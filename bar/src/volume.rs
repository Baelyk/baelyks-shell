use std::process::Command;

use log::warn;

pub async fn volume() -> Option<VolumeInfo> {
    get_info().ok()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    pub volume: u32,
    pub icon: &'static str,
}

fn get_info() -> Result<VolumeInfo, Box<dyn std::error::Error>> {
    let stdout = String::try_from(
        Command::new("wpctl")
            .arg("get-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .output()?
            .stdout,
    )?;

    let volume = stdout
        .matches(char::is_numeric)
        .collect::<String>()
        .parse::<u32>()?;

    let muted = stdout.contains("MUTED");

    let icon = icon(volume, muted);

    Ok(VolumeInfo { volume, icon })
}

pub fn toggle_mute() {
    if Command::new("wpctl")
        .arg("set-mute")
        .arg("@DEFAULT_AUDIO_SINK@")
        .arg("toggle")
        .output()
        .is_err()
    {
        warn!("Unable to toggle mute");
    }
}

pub fn increase_volume() {
    let output = Command::new("wpctl")
        .arg("set-volume")
        .arg("--limit")
        .arg("1.0")
        .arg("@DEFAULT_AUDIO_SINK@")
        .arg("1%+")
        .output();
    if !output.is_ok_and(|output| output.status.success()) {
        warn!("Unable to increase volume");
    }
}

pub fn decrease_volume() {
    let output = Command::new("wpctl")
        .arg("set-volume")
        .arg("@DEFAULT_AUDIO_SINK@")
        .arg("1%-")
        .output();
    if !output.is_ok_and(|output| output.status.success()) {
        warn!("Unable to decrease volume");
    }
}

fn icon(volume: u32, muted: bool) -> &'static str {
    if muted {
        "audio-volume-muted"
    } else if volume <= 33 {
        "audio-volume-low"
    } else if volume <= 66 {
        "audio-volume-medium"
    } else {
        "audio-volume-high"
    }
}
