use std::{os::unix::process::CommandExt, sync::Arc};

use baelyks_shell_lib::gruvbox;
use iced::{
    Element, Length, Subscription, Task, Theme,
    keyboard::{key, on_key_press, on_key_release},
    widget::{self},
};
use iced_layershell::{
    application, reexport::Anchor, settings::LayerShellSettings, to_layer_message,
};
use log::{debug, error, info, warn};

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
                widget::container(
                    widget::scrollable::Scrollable::with_direction(
                        iced::widget::column(
                            self.entries
                                .iter()
                                .enumerate()
                                .map(|(i, entry)| {
                                    let icon = entry.icon();
                                    let image: Option<Element<Message>> = icon.map(|icon| {
                                        if icon
                                            .extension()
                                            .is_some_and(|extension| extension == "svg")
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
                                        .wrapping(widget::text::Wrapping::WordOrGlyph);
                                    let row = widget::row![icon, widget::center_y(text)]
                                        .spacing(SIZE_TINY)
                                        .width(Length::Fill);

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
                        ),
                        widget::scrollable::Direction::Vertical(
                            widget::scrollable::Scrollbar::new()
                                .scroller_width(SIZE_TINY)
                                .spacing(SIZE_TINY - SIZE_BORDER),
                        ),
                    )
                    .style(|theme, status| {
                        let mut style = widget::scrollable::default(theme, status);
                        style.vertical_rail.background = None;
                        style.vertical_rail.scroller.border.width = 0.0;
                        style.vertical_rail.scroller.border.radius = 0.into();

                        style.vertical_rail.scroller.color = match status {
                            widget::scrollable::Status::Hovered {
                                is_vertical_scrollbar_hovered: true,
                                ..
                            } => gruvbox::LIGHT3,
                            widget::scrollable::Status::Dragged {
                                is_vertical_scrollbar_dragged: true,
                                ..
                            } => gruvbox::LIGHT4,
                            _ => gruvbox::LIGHT1,
                        };

                        style
                    })
                    .width(Length::Fill),
                )
                .style(|theme| widget::container::Style {
                    background: Some(theme.palette().background.into()),
                    border: iced::Border {
                        color: theme.palette().text,
                        width: SIZE_BORDER,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                })
                .padding(iced::Padding::new(SIZE_TINY)),
            )
        };
        widget::container(widget::column![search_bar, results].spacing(SIZE_MEDIUM))
            .max_height(HEIGHT)
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
                    self.entries = results;
                }
            },
            Message::RequestClose => tasks.push(iced::exit()),
            Message::SelectUp => {
                self.selected = self.selected.saturating_sub(1);
            }
            Message::SelectDown => {
                self.selected =
                    std::cmp::min(self.selected + 1, self.entries.len().saturating_sub(1));
            }
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
