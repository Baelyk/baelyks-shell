pub mod desktop_entries;
pub mod paths;

pub trait Entry: std::fmt::Debug + Send + Sync {
    fn icon(&self) -> Option<std::path::PathBuf>;

    fn name(&self) -> String;

    fn open(&self) -> Result<(), Box<dyn std::error::Error>>;

    fn matcher_column(&self) -> nucleo::Utf32String;
}
