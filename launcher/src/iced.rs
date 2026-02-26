use std::{os::unix::process::CommandExt, sync::Arc};

use iced::{
    Element, Length, Subscription, Task, Theme,
    keyboard::{self, key},
    widget::{self},
};
use iced_layershell::{
    application, reexport::Anchor, settings::LayerShellSettings, to_layer_message,
};
use log::{debug, error, info, trace, warn};

use crate::{providers::Entry, searcher, selectable_rows::SelectableRows};

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
    Select(usize),
    OpenSelected,
}

pub const SIZE_BORDER: f32 = 2.0;
pub const SIZE_TINY: f32 = 10.0;
pub const SIZE_MEDIUM: f32 = 30.0;
pub const HEIGHT: u32 = 1000;
pub const WIDTH: u32 = 2000;
pub const FONT: iced::Font = iced::Font::with_name("JetBrainsMono Nerd Font");

impl State {
    fn view(&self) -> impl Into<Element<'_, Message>> {
        const TEXT_SIZE: f32 = SIZE_MEDIUM;
        const ICON_SIZE: f32 = TEXT_SIZE * 1.2;
        let search_bar = widget::text_input("Search...", &self.input)
            .on_input(Message::ContentChanged)
            .on_submit(Message::OpenSelected)
            .size(SIZE_MEDIUM)
            .padding(SIZE_MEDIUM)
            .id("searchbar")
            .style(|theme, status| {
                let mut style = widget::text_input::default(theme, status);
                style.border = iced::Border {
                    color: theme.palette().text,
                    width: SIZE_BORDER,
                    radius: 0.0.into(),
                };
                style
            });
        let results = if self.entries.is_empty() {
            None
        } else {
            Some(
                widget::container(widget::stack!(
                    SelectableRows::with_rows(
                        self.entries
                            .iter()
                            .take(self.selected + 25)
                            .enumerate()
                            .map(|(i, entry)| {
                                let icon = entry.icon();
                                let image: Option<Element<Message>> = icon.map(|icon| {
                                    if icon.extension().is_some_and(|extension| extension == "svg")
                                    {
                                        iced::widget::svg(icon)
                                            .width(Length::Fill)
                                            .height(Length::Fill)
                                            .style(move |theme: &iced::Theme, _| {
                                                widget::svg::Style {
                                                    color: if i == self.selected {
                                                        Some(theme.palette().background)
                                                    } else {
                                                        None
                                                    },
                                                }
                                            })
                                            .into()
                                    } else {
                                        iced::widget::image(icon)
                                            .width(Length::Fill)
                                            .height(Length::Fill)
                                            .into()
                                    }
                                });
                                let icon = widget::center(image)
                                    .padding(SIZE_TINY)
                                    .height(ICON_SIZE + SIZE_TINY)
                                    .width(ICON_SIZE + SIZE_TINY);

                                let text = entry
                                    .text()
                                    .size(TEXT_SIZE)
                                    .width(
                                        // TODO: better way to do this: Length::Fill forces row
                                        // wrapping
                                        WIDTH as f32
                                            // Icon
                                            - (ICON_SIZE + SIZE_TINY)
                                            // Row spacing
                                            - SIZE_TINY
                                            // Container padding
                                            - 2.0 * SIZE_TINY
                                            // Border
                                            - 2.0 * SIZE_BORDER,
                                    )
                                    .wrapping(widget::text::Wrapping::WordOrGlyph);
                                let row = widget::row![icon, widget::center_y(text)]
                                    .spacing(SIZE_TINY)
                                    .width(Length::Fill)
                                    .wrap();

                                widget::Container::new(row).style(move |theme| {
                                    let palette = theme.palette();
                                    let (text_color, background) = if i == self.selected {
                                        (Some(palette.background), Some(palette.text.into()))
                                    } else {
                                        (None, None)
                                    };
                                    widget::container::Style {
                                        text_color,
                                        background,
                                        ..Default::default()
                                    }
                                })
                            })
                            .map(Element::from),
                        ICON_SIZE + SIZE_TINY,
                        Box::new(Message::Select),
                    )
                    .padding(SIZE_TINY),
                    // Container with background colored border to hide render overflow
                    widget::container(None::<Element<'_, Message>>)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|theme: &Theme| {
                            widget::container::Style {
                                border: iced::Border {
                                    color: theme.palette().background,
                                    width: SIZE_TINY,
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        }),
                    // Container to provide the border
                    widget::container(None::<Element<'_, Message>>)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|theme: &Theme| widget::container::Style {
                            border: iced::Border {
                                color: theme.palette().text,
                                width: SIZE_BORDER,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                ))
                .style(|theme: &Theme| widget::container::Style {
                    background: Some(theme.palette().background.into()),
                    ..Default::default()
                }),
            )
        };
        widget::container(widget::column![search_bar, results].spacing(SIZE_MEDIUM))
            .max_height(HEIGHT)
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        trace!("Message: {message:?}");
        let mut tasks = vec![widget::operation::focus("searchbar")];

        match message {
            Message::ContentChanged(content) => {
                self.input = content.clone();
                if let Some(searcher) = &mut self.searcher {
                    searcher.send(searcher::Message::UpdatePattern(content));
                }
            }
            Message::Searcher(event) => match event {
                searcher::Event::Initialized((searcher, injector)) => {
                    self.searcher = Some(searcher);
                    let entry_injector = injector.clone();
                    tasks.push(iced_runtime::task::blocking(|_| {
                        crate::providers::desktop_entries::inject_entries(entry_injector);
                    }));
                    let path_injector = injector.clone();
                    tasks.push(iced_runtime::task::blocking(|_| {
                        crate::providers::paths::inject_paths(path_injector);
                    }));
                }
                searcher::Event::FoundResults(results) => {
                    debug!("Received {} results", results.len());
                    self.entries = results;
                }
            },
            Message::RequestClose => tasks.push(iced::exit()),
            Message::Select(selected) => self.selected = selected,
            Message::OpenSelected => {
                debug!("Opening {}!", self.selected);
                let mut command = self.entries[self.selected]
                    .open()
                    .expect("Error opening option");
                info!("Command: {command:?}");
                // exec only returns if something went wrong
                let error = command.exec();
                error!("Error opening {}: {}", self.selected, error);
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
            keyboard::listen().filter_map(|event| {
                let keyboard::Event::KeyReleased { key, .. } = event else {
                    return None;
                };

                match key {
                    keyboard::Key::Named(key::Named::Escape) => Some(Message::RequestClose),
                    _ => None,
                }
            }),
            //on_key_release(|key, _| match key {
            //    key::Key::Named(key::Named::Escape) => Some(Message::RequestClose),
            //    _ => None,
            //}),
        ];
        Subscription::batch(subscriptions)
    }

    fn namespace() -> String {
        String::from("Baelyk's Launcher")
    }

    fn style(&self, theme: &Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: iced::Color::TRANSPARENT,
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
            size: Some((WIDTH, HEIGHT)),
            keyboard_interactivity: iced_layershell::reexport::KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        default_font: FONT,
        ..Default::default()
    })
    .run()
}
