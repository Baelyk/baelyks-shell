use iced::futures::{SinkExt, Stream};

use crate::POLL_RATE_MS;

pub fn system() -> impl Stream<Item = SystemMessage> {
    iced::stream::channel(100, async move |mut output| {
        let refreshes = sysinfo::RefreshKind::nothing()
            .with_cpu(sysinfo::CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(sysinfo::MemoryRefreshKind::nothing().with_ram());
        let mut sys = sysinfo::System::new_with_specifics(refreshes);

        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_millis(POLL_RATE_MS)
                    .max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL),
            );
            let mut old_state = None;
            loop {
                let new_state = (&sys).into();
                if old_state != Some(new_state) {
                    output
                        .send(SystemMessage::Update(new_state))
                        .await
                        .expect("Unable to send update");

                    old_state = Some(new_state);
                }
                interval.tick().await;

                sys.refresh_specifics(refreshes);
            }
        });
    })
}

#[derive(Debug, Copy, Clone)]
pub enum SystemMessage {
    Update(SystemInfo),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SystemInfo {
    pub cpu: f32,
    pub memory: f32,
}

impl From<&sysinfo::System> for SystemInfo {
    fn from(system: &sysinfo::System) -> Self {
        Self {
            cpu: system.global_cpu_usage(),
            memory: (system.used_memory() * 100) as f32 / system.total_memory() as f32,
        }
    }
}
