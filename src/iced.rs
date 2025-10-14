use chrono::Local;
use iced::{
    Element, Length, Subscription, Task, Theme,
    widget::{self, Row, button, center_y, mouse_area, row, text},
    window,
};
use iced_layershell::{
    Settings, daemon, reexport::Anchor, settings::LayerShellSettings, to_layer_message,
};
use log::{debug, trace, warn};

use crate::{
    POLL_RATE_MS,
    battery::{self, BatteryInfo, BatteryMessage},
    sway::{InputInfo, SwayMessenger},
    system::{self, SystemInfo, SystemMessage},
    tray::{TrayItems, TrayMessage},
    volume::VolumeInfo,
};
use crate::{
    sway::{self, SwayMessage, WorkspaceInfo},
    volume,
};

const HEIGHT: u32 = 40;
const TEXT_SIZE: f32 = 20.0;
const SMALL: f32 = 12.0;
const MEDIUM: f32 = 24.0;
const BIG: f32 = 36.0;

pub fn run() -> Result<(), iced_layershell::Error> {
    daemon(State::default, State::namespace, State::update, State::view)
        .subscription(State::subscription)
        .style(State::style)
        .theme(State::theme)
        .settings(Settings {
            layer_settings: LayerShellSettings {
                anchor: Anchor::Top | Anchor::Left | Anchor::Right,
                size: Some((0, HEIGHT)),
                exclusive_zone: HEIGHT as i32,
                start_mode: iced_layershell::settings::StartMode::AllScreens,
                ..Default::default()
            },
            default_font: iced::Font::with_name("JetBrainsMono Nerd Font"),
            ..Default::default()
        })
        .run()
}

#[derive(Default)]
struct State {
    clock_hovered: bool,
    workspaces: Vec<WorkspaceInfo>,
    sway_messenger: Option<SwayMessenger>,
    battery: Option<BatteryInfo>,
    battery_hovered: bool,
    volume: Option<VolumeInfo>,
    tray_items: Option<TrayItems>,
    system_info: Option<SystemInfo>,
    system_hovered: bool,
    input: Option<InputInfo>,
}

#[to_layer_message(multi)]
#[derive(Clone, Debug)]
enum Message {
    Tick,
    ClockHover(bool),
    Sway(SwayMessage),
    SwitchWorkspace(i32),
    Battery(BatteryMessage),
    BatteryHover(bool),
    Volume(Option<VolumeInfo>),
    VolumeToggleMute,
    VolumeScroll(iced::mouse::ScrollDelta),
    Tray(TrayMessage),
    System(SystemMessage),
    SystemHover(bool),
}

fn icon(icon: &str) -> Option<Element<Message>> {
    let icon = crate::freedesktop::find_icon_path(icon)?;
    Some(
        widget::svg(icon)
            .width(Length::Fixed(BIG))
            .height(Length::Fixed(BIG))
            .into(),
    )
}

impl State {
    fn namespace() -> String {
        String::from("Bar")
    }

    fn workspaces(&self) -> Element<Message> {
        center_y(
            Row::from_iter(self.workspaces.iter().map(|info| {
                button("")
                    .on_press(Message::SwitchWorkspace(info.num))
                    .style(|theme: &Theme, _| iced::widget::button::Style {
                        background: if info.urgent {
                            Some(theme.palette().danger.into())
                        } else if info.focused {
                            Some(theme.palette().primary.into())
                        } else if info.nonempty {
                            Some(theme.palette().text.into())
                        } else {
                            None
                        },
                        border: iced::Border::default()
                            .width(2)
                            .rounded(3)
                            .color(if info.urgent {
                                theme.palette().danger
                            } else if info.focused || info.visible {
                                theme.palette().primary
                            } else {
                                theme.palette().text
                            }),
                        ..Default::default()
                    })
                    .width(SMALL)
                    .height(SMALL)
                    .into()
            }))
            .spacing(MEDIUM)
            .padding([0.0, MEDIUM]),
        )
        .into()
    }

    fn clock(&self) -> Element<Message> {
        let format = if self.clock_hovered { "%c" } else { "%H:%M" };
        let time = Local::now().format(format);

        // Red clock when building in debug
        let style = move |_: &Theme| widget::text::Style {
            color: if cfg!(debug_assertions) {
                Some([1.0, 0.0, 0.0].into())
            } else {
                None
            },
        };

        mouse_area(
            center_y(text(time.to_string()).size(TEXT_SIZE).style(style)).padding([0.0, SMALL]),
        )
        .on_enter(Message::ClockHover(true))
        .on_exit(Message::ClockHover(false))
        .into()
    }

    fn battery(&self) -> Option<Element<Message>> {
        let info = self.battery?;
        let battery_icon = icon(info.icon)?;

        let content = Row::new()
            .push(center_y(battery_icon))
            .push_maybe(if self.battery_hovered {
                Some(center_y(text(format!("{}%", info.charge)).size(TEXT_SIZE)))
            } else {
                None
            })
            .spacing(SMALL);

        Some(
            mouse_area(center_y(content).padding([0.0, SMALL]))
                .on_enter(Message::BatteryHover(true))
                .on_exit(Message::BatteryHover(false))
                .into(),
        )
    }

    fn volume(&self) -> Option<Element<Message>> {
        let info = self.volume?;

        let icon = icon(info.icon)?;

        Some(
            mouse_area(
                row![
                    center_y(icon),
                    center_y(text(format!("{:>3}%", info.volume)).size(TEXT_SIZE))
                ]
                .padding([0.0, SMALL]),
            )
            .on_press(Message::VolumeToggleMute)
            .on_scroll(Message::VolumeScroll)
            .into(),
        )
    }

    fn tray(&self) -> Option<Element<Message>> {
        let Some(items) = &self.tray_items else {
            return None;
        };

        Some(
            center_y(
                Row::from_iter(items.values().map(|item| {
                    let icon: Element<Message> =
                        if item.icon.extension().is_some_and(|ext| ext == "svg") {
                            widget::svg(item.icon.clone()).into()
                        } else {
                            widget::image(item.icon.clone()).into()
                        };

                    widget::tooltip(icon, text(item.title.clone()), Default::default()).into()
                }))
                .height(HEIGHT as f32 / 2.0),
            )
            .into(),
        )
    }

    fn system(&self) -> Option<Element<Message>> {
        let info = self.system_info?;

        let cpu_icon = if info.cpu <= 20.0 {
            icon("indicator-cpufreq")
        } else if info.cpu <= 40.0 {
            icon("indicator-cpufreq-25")
        } else if info.cpu <= 60.0 {
            icon("indicator-cpufreq-50")
        } else if info.cpu <= 80.0 {
            icon("indicator-cpufreq-75")
        } else {
            icon("indicator-cpufreq-100")
        }?;

        let row = if self.system_hovered {
            Row::new().push(center_y(
                text(format!("{:>2.0}% {:>2.0}%", info.memory, info.cpu)).size(TEXT_SIZE),
            ))
        } else {
            Row::new()
        }
        .push(center_y(cpu_icon));

        Some(
            mouse_area(center_y(row).padding([0.0, SMALL]))
                .on_enter(Message::SystemHover(true))
                .on_exit(Message::SystemHover(false))
                .into(),
        )
    }

    fn input(&self) -> Option<Element<Message>> {
        Some(
            center_y(icon(self.input?.icon)?)
                .padding([0.0, SMALL])
                .into(),
        )
    }

    fn view(&self, _: window::Id) -> Element<Message> {
        let left = row![self.workspaces()];

        let right = Row::new()
            .spacing(SMALL)
            .push_maybe(self.tray())
            .push_maybe(self.system())
            .push_maybe(self.input())
            .push_maybe(self.volume())
            .push_maybe(self.battery())
            .push(self.clock());
        let right = widget::right(right);

        row![left, right].width(Length::Fill).into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        trace!("Update {:#?}", message);
        match message {
            Message::Tick => Task::none(),
            Message::ClockHover(hovered) => {
                self.clock_hovered = hovered;
                Task::none()
            }
            Message::Sway(message) => {
                match message {
                    SwayMessage::Initialized(sway_messenger) => {
                        self.sway_messenger = Some(sway_messenger)
                    }
                    SwayMessage::Workspaces(workspaces) => {
                        self.workspaces = workspaces;
                    }
                    SwayMessage::Input(input) => {
                        self.input = Some(input);
                    }
                }
                Task::none()
            }
            Message::SwitchWorkspace(num) => match &mut self.sway_messenger {
                Some(sway_messenger) => {
                    sway_messenger.switch_workspace(num);
                    Task::none()
                }
                None => {
                    warn!("Unable to send SwitchWorkspace({num}), SwayMessenger uninitialized");
                    Task::none()
                }
            },
            Message::Battery(message) => {
                match message {
                    BatteryMessage::Update(info) => {
                        self.battery = Some(info);
                    }
                }
                Task::none()
            }
            Message::BatteryHover(hovered) => {
                self.battery_hovered = hovered;
                Task::none()
            }
            Message::Volume(info) => {
                if self.volume != info {
                    self.volume = info;
                }
                Task::none()
            }
            Message::VolumeToggleMute => {
                volume::toggle_mute();
                Task::none()
            }
            Message::VolumeScroll(delta) => {
                if let iced::mouse::ScrollDelta::Pixels { x: _, y: delta } = delta {
                    if delta > 1.0 {
                        volume::decrease_volume();
                    } else if delta < -1.0 {
                        volume::increase_volume();
                    }
                }
                Task::future(volume::volume()).map(Message::Volume)
            }
            Message::Tray(message) => {
                debug!("TrayMessage: {:#?}", message);
                match message {
                    TrayMessage::Initialized(tray_items) => self.tray_items = Some(tray_items),
                    TrayMessage::Add(dest, item) => {
                        if let Some(tray_items) = &mut self.tray_items {
                            tray_items.insert(dest, item);
                        } else {
                            warn!("Unable to add tray item to uninitialized tray");
                        }
                    }
                    TrayMessage::Remove(dest) => {
                        if let Some(tray_items) = &mut self.tray_items {
                            tray_items.remove(&dest);
                        } else {
                            warn!("Unable to remove tray item from uninitialized tray");
                        }
                    }
                }
                Task::none()
            }
            Message::System(message) => {
                match message {
                    SystemMessage::Update(info) => self.system_info = Some(info),
                }
                Task::none()
            }
            Message::SystemHover(hovered) => {
                self.system_hovered = hovered;
                Task::none()
            }
            _ => {
                warn!("Unexpected message {:?}", message);
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick =
            iced::time::every(iced::time::Duration::from_millis(1000)).map(|_| Message::Tick);
        let sway = Subscription::run(sway::sway).map(Message::Sway);
        let battery = Subscription::run(battery::battery).map(Message::Battery);
        let volume = iced::time::repeat(
            volume::volume,
            iced::time::Duration::from_millis(POLL_RATE_MS),
        )
        .map(Message::Volume);
        let system = Subscription::run(system::system).map(Message::System);
        //let tray = Subscription::run(tray::tray).map(Message::Tray);
        Subscription::batch([tick, sway, battery, volume, system])
    }

    fn style(&self, theme: &Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: theme.palette().background,
            text_color: theme.palette().text,
        }
    }

    fn theme(&self, _: window::Id) -> Theme {
        iced::Theme::custom(
            "Gruvbox Dark".to_string(),
            iced::theme::Palette {
                text: iced::color!(0xebdbb2),
                ..iced::theme::Palette::GRUVBOX_DARK
            },
        )
    }
}
