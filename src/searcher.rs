use derive_more::Debug;
use freedesktop_desktop_entry::DesktopEntry;
use iced::futures::channel::mpsc;
use iced::futures::{select, SinkExt, Stream, StreamExt};
use walkdir::DirEntry;

use crate::LOCALES;

#[derive(Debug, Clone)]
pub enum SearchItem {
    DesktopEntry(DesktopEntry<'static>),
    DirEntry(DirEntry),
}

impl std::fmt::Display for SearchItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchItem::DesktopEntry(entry) => write!(
                f,
                "{}",
                entry
                    .name(&LOCALES)
                    .or(entry.generic_name(&LOCALES))
                    .unwrap_or_default()
            ),
            SearchItem::DirEntry(entry) => write!(f, "{}", entry.path().display()),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Icon {
    Svg(std::path::PathBuf),
    Raster(std::path::PathBuf),
    None,
}

impl SearchItem {
    pub(crate) fn search_data(&self) -> nucleo::Utf32String {
        match self {
            SearchItem::DesktopEntry(entry) => {
                let name = entry.name(&LOCALES).unwrap_or_default();
                let comment = entry.comment(&LOCALES).unwrap_or_default();
                let generic_name = entry.generic_name(&LOCALES).unwrap_or_default();
                format!("{name} {comment} {generic_name}").into()
            }
            SearchItem::DirEntry(entry) => entry.path().to_string_lossy().into(),
        }
    }

    pub(crate) fn open(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Self::DesktopEntry(entry) => {
                std::process::Command::new("sh")
                    .arg("-c")
                    .args(entry.parse_exec()?)
                    .spawn()?;
            }
            Self::DirEntry(entry) => {
                std::process::Command::new("xdg-open")
                    .arg(entry.path())
                    .spawn()?;
            }
        }

        Ok(())
    }

    pub(crate) fn icon(&self) -> Icon {
        // TODO: Theme settings?
        let theme = "Gruvbox-Plus-Dark";

        let name = match self {
            Self::DesktopEntry(entry) => entry.icon().unwrap_or(""),
            Self::DirEntry(entry) => {
                if let Some(mime) = mime_guess::from_path(entry.path()).first() {
                    &mime.essence_str().replace("/", "-")
                } else {
                    "application-blank"
                }
            }
        };

        if let Some(path) = freedesktop_icons::lookup(name)
            .with_cache()
            .force_svg()
            .with_theme(theme)
            .find()
        {
            return Icon::Svg(path);
        }

        if let Some(path) = freedesktop_icons::lookup(name)
            .with_cache()
            .with_size(100)
            .with_theme(theme)
            .find()
        {
            return Icon::Raster(path);
        }

        Icon::None
    }
}

pub fn nucleo() -> impl Stream<Item = Event> {
    iced::stream::channel(100, |mut output| async move {
        // Create a new Nucleo worker
        let (notify_sender, mut notifier) = mpsc::channel(100);
        let mut notify_on_patterns = notify_sender.clone();
        let notify = std::sync::Arc::new(move || {
            let _ = iced::futures::executor::block_on(notify_sender.clone().send(()));
        });
        let mut nucleo: nucleo::Nucleo<SearchItem> =
            nucleo::Nucleo::new(nucleo::Config::DEFAULT, notify, None, 1);

        // Create the channel to communicate with the GUI
        let (sender, mut receiver) = mpsc::channel(100);

        // Let the GUI know that the searcher is initialized
        let _ = output
            .send(Event::Initialized((Searcher(sender), nucleo.injector())))
            .await;

        let mut initialized = false;

        loop {
            select! {
                message = receiver.select_next_some() => {
                    println!("Received {:?}", message);
                    initialized = true;
                    match message {
                        Message::UpdatePattern(pattern) => {
                            nucleo.pattern.reparse(
                                0,
                                &pattern,
                                nucleo::pattern::CaseMatching::Smart,
                                nucleo::pattern::Normalization::Smart,
                                false,
                            );
                            let _ = notify_on_patterns.send(()).await;
                        }
                    }
                }
                _ = notifier.select_next_some() => {
                    if !initialized {
                        continue;
                    }
                    println!("Searching...");
                    let status = dbg!(nucleo.tick(5));

                    if status.changed {
                        let snapshot = nucleo.snapshot();
                        println!("Found {} results", snapshot.matched_item_count());
                        let items = std::cmp::min(20, snapshot.matched_item_count());
                        let range = 0..items;
                        let results: Vec<SearchItem> = snapshot
                            .matched_items(range)
                            .map(|item| item.data.clone())
                            .collect();

                        println!("Sending {} results", results.len());
                        let _ = output.send(Event::FoundResults(results)).await;
                    }
                }
            }
        }
    })
}

#[derive(Debug, Clone)]
pub enum Event {
    #[debug("Initialized(Searcher, Injector)")]
    Initialized((Searcher, nucleo::Injector<SearchItem>)),
    FoundResults(Vec<SearchItem>),
}

#[derive(Debug, Clone)]
pub struct Searcher(mpsc::Sender<Message>);
impl Searcher {
    pub fn send(&mut self, message: Message) {
        println!("Sending a message");
        self.0
            .try_send(message)
            .expect("Unable to send message to Searcher");
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdatePattern(String),
}
