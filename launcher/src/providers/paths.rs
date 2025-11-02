use std::sync::Arc;

use baelyks_shell_lib::{freedesktop::find_icon_path, gruvbox};
use iced::widget::{rich_text, span};
use walkdir::DirEntry;

use crate::{iced::Message, providers::Entry};

impl Entry for DirEntry {
    fn icon(&self) -> Option<std::path::PathBuf> {
        let icon = if let Some(mime) = mime_guess::from_path(self.path()).first() {
            &mime.essence_str().replace("/", "-")
        } else {
            "application-blank"
        };
        find_icon_path(icon, None)
    }

    fn text(&self) -> iced::widget::text::Rich<'_, (), Message> {
        let name = self.file_name().to_string_lossy();
        let path = self
            .path()
            .parent()
            .map(|parent| parent.to_string_lossy())
            .unwrap_or_default();
        rich_text![
            span(format!("{path}/")).color(gruvbox::GRAY_244),
            span(name)
        ]
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
                cols[1] = entry.matcher_column();
            });
        });
}
