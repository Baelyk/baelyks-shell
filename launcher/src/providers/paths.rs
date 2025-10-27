use std::sync::Arc;

use baelyks_shell_lib::freedesktop::find_icon_path;
use walkdir::DirEntry;

use crate::providers::Entry;

impl Entry for DirEntry {
    fn icon(&self) -> Option<std::path::PathBuf> {
        let icon = if let Some(mime) = mime_guess::from_path(self.path()).first() {
            &mime.essence_str().replace("/", "-")
        } else {
            "application-blank"
        };
        find_icon_path(icon, None)
    }

    fn name(&self) -> String {
        self.path().to_string_lossy().to_string()
    }

    fn open(&self) -> Result<std::process::Command, Box<dyn std::error::Error>> {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(self.path());
        Ok(command)
    }

    fn matcher_column(&self) -> nucleo::Utf32String {
        self.path().to_string_lossy().into()
    }
}

pub fn inject_paths(injector: nucleo::Injector<Arc<dyn Entry>>) {
    walkdir::WalkDir::new("/home/baelyk/")
        .into_iter()
        .filter_entry(|entry| {
            !entry
                .file_name()
                .to_str()
                .map(|s| s.starts_with("."))
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            let Ok(entry) = entry else {
                return None;
            };
            if entry.file_type().is_file() {
                return Some(entry);
            }
            None
        })
        .for_each(|entry| {
            let entry = Arc::new(entry);
            injector.push(entry, |entry, cols| {
                cols[0] = entry.matcher_column();
            });
        });
}
