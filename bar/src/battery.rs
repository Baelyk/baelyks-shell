use iced::futures::{SinkExt, Stream};
use log::warn;

use crate::POLL_RATE_MS;

pub fn battery() -> impl Stream<Item = BatteryMessage> {
    iced::stream::channel(100, async move |mut output| {
        let Ok(manager) = starship_battery::Manager::new() else {
            warn!("Unable to get battery manager");
            return;
        };
        let Ok(mut batteries) = manager.batteries() else {
            warn!("Unable to get batteries");
            return;
        };
        let Some(Ok(mut battery)) = batteries.next() else {
            warn!("Unable to get battery");
            return;
        };

        tokio::task::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(POLL_RATE_MS));
            let mut old_state = None;
            loop {
                let new_state = (&battery).into();
                if old_state != Some(new_state) {
                    output
                        .send(BatteryMessage::Update(new_state))
                        .await
                        .expect("Unable to send update");

                    old_state = Some(new_state);
                }
                interval.tick().await;

                if manager.refresh(&mut battery).is_err() {
                    warn!("Unable to refresh battery");
                    break;
                }
            }
        });
    })
}

#[derive(Debug, Copy, Clone)]
pub enum BatteryMessage {
    Update(BatteryInfo),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BatteryInfo {
    pub charge: u32,
    pub icon: &'static str,
}

impl From<&starship_battery::Battery> for BatteryInfo {
    fn from(battery: &starship_battery::Battery) -> Self {
        // Get charge as a two-digit percent
        let charge = (battery.state_of_charge().value * 100.0).floor() as u32;
        Self {
            charge,
            icon: battery_icon(charge, battery.state()),
        }
    }
}

fn battery_icon(charge: u32, state: starship_battery::State) -> &'static str {
    let index = (charge / 10) as usize;
    match state {
        starship_battery::State::Charging => [
            "battery-000-charging",
            "battery-010-charging",
            "battery-020-charging",
            "battery-030-charging",
            "battery-040-charging",
            "battery-050-charging",
            "battery-060-charging",
            "battery-070-charging",
            "battery-080-charging",
            "battery-090-charging",
            "battery-100-charging",
        ][index],
        _ => [
            "battery-000",
            "battery-010",
            "battery-020",
            "battery-030",
            "battery-040",
            "battery-050",
            "battery-060",
            "battery-070",
            "battery-080",
            "battery-090",
            "battery-100",
        ][index],
    }
}
