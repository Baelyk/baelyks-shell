use crate::iced::Message;

pub mod desktop_entries;
pub mod paths;

pub trait Entry: std::fmt::Debug + Send + Sync {
    fn icon(&self) -> Option<std::path::PathBuf>;

    fn text(&self) -> iced::widget::text::Rich<'_, (), Message>;

    fn open(&self) -> Result<std::process::Command, Box<dyn std::error::Error>>;

    fn matcher_column(&self) -> nucleo::Utf32String;
}
