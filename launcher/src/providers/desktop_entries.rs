use std::sync::Arc;

use baelyks_shell_lib::{
    freedesktop::{LOCALES, find_icon_path},
    gruvbox,
};
use freedesktop_desktop_entry::DesktopEntry;
use iced::widget::{rich_text, span};

use crate::{
    iced::{FONT, Message},
    providers::Entry,
};

impl Entry for DesktopEntry {
    fn icon(&self) -> Option<std::path::PathBuf> {
        let icon = self.icon().unwrap_or(&*self.appid);
        find_icon_path(icon, Some("Applications"))
    }

    fn text(&self) -> iced::widget::text::Rich<'_, (), Message> {
        rich_text![
            // Name
            span(self.name(&LOCALES).unwrap_or_default()),
            // Comment
            self.comment(&LOCALES)
                .map(|comment| span(format!(" ({})", comment))
                    .font(iced::Font {
                        style: iced::font::Style::Italic,
                        ..FONT
                    })
                    .color(gruvbox::GRAY_244))
                .unwrap_or_default()
        ]
    }

    fn open(&self) -> Result<std::process::Command, Box<dyn std::error::Error>> {
        let mut command = if self.terminal() {
            // TODO equivalent of sensible-terminal
            let mut command = std::process::Command::new("wezterm");
            command.arg("-e");
            command
        } else {
            let mut command = std::process::Command::new("sh");
            command.arg("-c");
            command
        };
        command.args(self.parse_exec()?);
        Ok(command)
    }

    fn matcher_column(&self) -> nucleo::Utf32String {
        let name = self.name(&LOCALES).unwrap_or_default();
        let comment = self.comment(&LOCALES).unwrap_or_default();
        let generic_name = self.generic_name(&LOCALES).unwrap_or_default();
        let exec = self.exec().unwrap_or_default();
        format!("{name} {comment} {generic_name} {exec}").into()
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
