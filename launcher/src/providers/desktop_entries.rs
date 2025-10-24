use std::sync::Arc;

use baelyks_shell_lib::freedesktop::{LOCALES, find_icon_path};
use freedesktop_desktop_entry::DesktopEntry;

use crate::providers::Entry;

impl Entry for DesktopEntry {
    fn icon(&self) -> Option<std::path::PathBuf> {
        let icon = self.icon().unwrap_or(&*self.appid);
        find_icon_path(icon, Some("Applications"))
    }

    fn name(&self) -> String {
        self.name(&LOCALES).unwrap_or_default().to_string()
    }

    fn open(&self) -> Result<std::process::Command, Box<dyn std::error::Error>> {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").args(self.parse_exec()?);
        Ok(command)
    }

    fn matcher_column(&self) -> nucleo::Utf32String {
        let name = self.name(&LOCALES).unwrap_or_default();
        let comment = self.comment(&LOCALES).unwrap_or_default();
        let generic_name = self.generic_name(&LOCALES).unwrap_or_default();
        format!("{name} {comment} {generic_name}").into()
    }
}

pub fn inject_entries(injector: nucleo::Injector<Arc<dyn Entry>>) {
    baelyks_shell_lib::freedesktop::get_desktop_entries()
        .into_iter()
        .for_each(|entry| {
            let entry = Arc::new(entry);
            injector.push(entry, |entry, cols| {
                cols[0] = entry.matcher_column();
            });
        });
}
