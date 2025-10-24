use std::{os::unix::process::CommandExt, sync::Arc};

use iced::{
    Element, Length, Subscription, Task, Theme,
    keyboard::{key, on_key_press, on_key_release},
    widget::{self, container},
    window,
};
use iced_layershell::{
    application, reexport::Anchor, settings::LayerShellSettings, to_layer_message,
};
use log::{debug, error, warn};

use crate::{providers::Entry, searcher};

#[derive(Default)]
struct State {
    input: String,
    searcher: Option<searcher::Searcher>,
    entries: Vec<Arc<dyn Entry>>,
    selected: usize,
}

#[to_layer_message]
#[derive(Clone, Debug)]
pub enum Message {
    ContentChanged(String),
    Searcher(searcher::Event),
    RequestClose,
    SelectUp,
    SelectDown,
    OpenSelected,

    TaskWindowOpen(window::Id),
    TaskWindowClose,
}

pub const SIZE_SMALL: f32 = SIZE_MEDIUM / 2.0;
pub const SIZE_MEDIUM: f32 = 35.0;
pub const HEIGHT: u32 = 1000;

impl State {
    fn view(&self) -> widget::Column<'_, Message> {
        const TEXT_SIZE: f32 = SIZE_MEDIUM;
        const ICON_SIZE: f32 = TEXT_SIZE * 1.2;
        let search_bar = widget::text_input("Search...", &self.input)
            .on_input(Message::ContentChanged)
            .on_submit(Message::OpenSelected)
            .size(SIZE_MEDIUM)
            .id("searchbar");
        let results = iced::widget::scrollable(
            iced::widget::column(
                self.entries
                    .iter()
                    .enumerate()
                    .map(|(i, entry)| {
                        let icon = entry.icon();
                        let image: Option<Element<Message>> = icon.map(|icon| {
                            if icon.extension().is_some_and(|extension| extension == "svg") {
                                iced::widget::svg(icon)
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .into()
                            } else {
                                iced::widget::image(icon)
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .into()
                            }
                        });
                        let icon = container(image).width(ICON_SIZE).height(ICON_SIZE);

                        let name = iced::widget::text(entry.name()).size(TEXT_SIZE);
                        let row = widget::row![icon, name]
                            .height(iced::Length::Shrink)
                            .width(iced::Length::Fill)
                            .padding([0.0, SIZE_SMALL])
                            .spacing(SIZE_SMALL)
                            .wrap();

                        widget::Container::new(row).style(move |theme| {
                            let background = if i == self.selected {
                                theme.palette().primary
                            } else {
                                theme.palette().background
                            };
                            widget::container::background(background)
                        })
                    })
                    .map(Element::from),
            )
            .spacing(10),
        )
        .height(iced::Fill);
        widget::column![search_bar, results]
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        debug!("Message: {message:?}");
        let mut tasks = vec![widget::operation::focus("searchbar")];

        match message {
            Message::ContentChanged(content) => {
                self.input = content.clone();
                if let Some(searcher) = &mut self.searcher {
                    searcher.send(searcher::Message::UpdatePattern(content));
                }
            }
            Message::Searcher(event) => {
                match event {
                    searcher::Event::Initialized((searcher, injector)) => {
                        self.searcher = Some(searcher);
                        // TODO: Not this
                        crate::providers::desktop_entries::inject_entries(injector.clone());
                        crate::providers::paths::inject_paths(injector.clone());
                    }
                    searcher::Event::FoundResults(results) => {
                        self.entries = results;
                    }
                }
            }
            Message::RequestClose => tasks.push(iced::exit()),
            Message::SelectUp => {
                self.selected = self.selected.saturating_sub(1);
            }
            Message::SelectDown => {
                self.selected = std::cmp::min(self.selected + 1, self.entries.len() - 1);
            }
            Message::OpenSelected => {
                println!("Opening {}!", self.selected);
                let mut command = self.entries[self.selected]
                    .open()
                    .expect("Error opening option");
                debug!("Command: {command:?}");
                // exec only returns if something went wrong
                let error = command.exec();
                error!("Error opening {}: {}", self.selected, error);
            }
            Message::TaskWindowOpen(id) => {
                println!("Window {} opening", id);
            }
            Message::TaskWindowClose => {
                println!("Window closing");
            }
            _ => {
                warn!("Unexpected message {:?}", message);
            }
        }

        Task::batch(tasks)
    }

    fn subscription(&self) -> Subscription<Message> {
        let subscriptions = [
            Subscription::run(searcher::nucleo).map(Message::Searcher),
            on_key_press(|key, _| match key {
                key::Key::Named(key::Named::ArrowUp) => Some(Message::SelectUp),
                key::Key::Named(key::Named::ArrowDown) => Some(Message::SelectDown),
                _ => None,
            }),
            on_key_release(|key, _| match key {
                key::Key::Named(key::Named::Escape) => Some(Message::RequestClose),
                _ => None,
            }),
        ];
        Subscription::batch(subscriptions)
    }

    fn namespace() -> String {
        String::from("Baelyk's Launcher")
    }

    fn style(&self, theme: &Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: theme.palette().background,
            text_color: theme.palette().text,
        }
    }

    fn theme(&self) -> Theme {
        iced::Theme::custom(
            "Gruvbox Dark".to_string(),
            iced::theme::Palette {
                text: iced::color!(0xebdbb2),
                ..iced::theme::Palette::GRUVBOX_DARK
            },
        )
    }
}

pub fn run() -> Result<(), iced_layershell::Error> {
    application(
        || (State::default(), widget::operation::focus("searchbar")),
        State::namespace,
        State::update,
        State::view,
    )
    .subscription(State::subscription)
    .style(State::style)
    .theme(State::theme)
    .settings(iced_layershell::Settings {
        layer_settings: LayerShellSettings {
            anchor: Anchor::Left | Anchor::Right,
            size: Some((1000, HEIGHT)),
            keyboard_interactivity: iced_layershell::reexport::KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        default_font: iced::Font::with_name("JetBrainsMono Nerd Font"),
        ..Default::default()
    })
    .run()
}
