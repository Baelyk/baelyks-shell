use iced::futures::{
    SinkExt, Stream, StreamExt,
    channel::mpsc::{self, Sender},
    select,
};
use log::{error, warn};
use swayipc_async::{Event, EventType};

pub fn sway() -> impl Stream<Item = SwayMessage> {
    iced::stream::channel(100, async move |mut output| {
        // Create the channel to communicate with the GUI
        let (sender, mut receiver) = mpsc::channel(100);

        let mut connection = swayipc_async::Connection::new()
            .await
            .expect("Unable to connect to sway");

        let workspaces = workspaces_info(&mut connection).await;
        output
            .send(SwayMessage::Workspaces(workspaces))
            .await
            .unwrap();

        let mut events = swayipc_async::Connection::new()
            .await
            .unwrap()
            .subscribe([EventType::Workspace, EventType::Input])
            .await
            .unwrap()
            .fuse();

        output
            .send(SwayMessage::Initialized(SwayMessenger(sender)))
            .await
            .unwrap();

        loop {
            select! {
                event = events.select_next_some() => {
                    let Ok(event) = event else {
                        continue;
                    };
                    match event {
                        Event::Workspace(_) => {
                            let workspaces = workspaces_info(&mut connection).await;
                            output
                                .send(SwayMessage::Workspaces(workspaces))
                                .await
                                .unwrap();
                        }
                        Event::Input(event) => {
                            if let Some(layout) = event.input.xkb_active_layout_name {
                                let icon = match layout.as_str() {
                                    "English (US)" => "indicator-keyboard-En",
                                    "Spanish" => "indicator-keyboard-Es",
                                    _ => {
                                        warn!("Unknown keyboard layout {}", layout);
                                        "indicator-keyboard"
                                    }
                                };
                                let input = InputInfo { icon };
                                output.send(SwayMessage::Input(input)).await.unwrap();
                            }
                        }
                        _ => warn!("Unexpected event {:?}", event),
                    }
                }
                task = receiver.select_next_some() => {
                    match task {
                        SwayTask::SwitchWorkspace(num) => {
                            swayipc_async::Connection::new()
                                .await
                                .unwrap()
                                .run_command(format!("workspace number {num}"))
                                .await
                                .unwrap();
                        }
                    }
                }
            }
        }
    })
}

#[derive(Debug, Clone)]
pub enum SwayMessage {
    Initialized(SwayMessenger),
    Workspaces(Vec<WorkspaceInfo>),
    Input(InputInfo),
}

#[derive(Debug, Copy, Clone)]
pub struct WorkspaceInfo {
    pub num: i32,
    pub visible: bool,
    pub focused: bool,
    pub urgent: bool,
    pub nonempty: bool,
}

impl WorkspaceInfo {
    fn empty(num: i32) -> Self {
        Self {
            num,
            visible: false,
            focused: false,
            urgent: false,
            nonempty: false,
        }
    }
}

impl From<&swayipc_async::Workspace> for WorkspaceInfo {
    fn from(workspace: &swayipc_async::Workspace) -> Self {
        Self {
            num: workspace.num,
            visible: workspace.visible,
            focused: workspace.focused,
            urgent: workspace.urgent,
            nonempty: true,
        }
    }
}

const WORKSPACES: usize = 10;
async fn workspaces_info(connection: &mut swayipc_async::Connection) -> Vec<WorkspaceInfo> {
    // Get sway workspaces
    let sway_workspaces = connection.get_workspaces().await.unwrap();

    // Create placeholder workspaces
    let mut workspaces: Vec<WorkspaceInfo> = (0..WORKSPACES)
        .map(|num| WorkspaceInfo::empty(num as i32))
        .collect();

    // Update the placeholders with the existing workspaces
    sway_workspaces.iter().for_each(|workspace| {
        if workspace.num >= 0 && (workspace.num as usize) < WORKSPACES {
            workspaces[workspace.num as usize] = workspace.into();
        }
    });

    // Workspace 0 at the end
    let zero = workspaces.remove(0);
    workspaces.push(zero);

    workspaces
}

#[derive(Debug, Copy, Clone)]
pub struct InputInfo {
    pub icon: &'static str,
}

#[derive(Debug, Clone)]
pub struct SwayMessenger(Sender<SwayTask>);
#[derive(Debug, Copy, Clone)]
enum SwayTask {
    SwitchWorkspace(i32),
}
impl SwayMessenger {
    pub fn switch_workspace(&mut self, num: i32) {
        if self.0.try_send(SwayTask::SwitchWorkspace(num)).is_err() {
            error!("Unable to send SwitchWorkspace({num}) task");
        }
    }
}
