use derive_more::Debug;
use iced::{
    keyboard::{key, on_key_press, on_key_release},
    widget, window, Element, Subscription,
};
use searcher::SearchItem;

mod searcher;

fn inject_entries(injector: nucleo::Injector<SearchItem>) {
    baelyks_shell_lib::freedesktop::get_desktop_entries()
        .into_iter()
        .for_each(|entry| {
            let item = SearchItem::DesktopEntry(entry);

            injector.push(item, |item, cols| {
                cols[0] = item.search_data();
            });
        });
}

fn inject_paths(injector: nucleo::Injector<SearchItem>) {
    walkdir::WalkDir::new("/home/baelyk/")
        .into_iter()
        .filter_map(|entry| {
            let Ok(entry) = entry else {
                return None;
            };
            if entry.file_type().is_file() {
                return Some(entry);
            }
            return None;
        })
        .for_each(|entry| {
            let item = SearchItem::DirEntry(entry);

            injector.push(item, |item, cols| {
                cols[0] = item.search_data();
            });
        });
    println!("injected {} paths!", injector.injected_items());
}

struct State {
    input: String,
    searcher: SearcherState,
    results: Vec<searcher::SearchItem>,
    selected: usize,
    theme: iced::Theme,
}

enum SearcherState {
    Unitialized,
    Initialized(searcher::Searcher),
}

impl Default for State {
    fn default() -> Self {
        let theme = iced::Theme::custom(
            "Gruvbox Dark".into(),
            iced::theme::Palette {
                text: iced::color!(0xebdbb2),
                ..iced::theme::Palette::GRUVBOX_DARK
            },
        );

        State {
            input: String::new(),
            searcher: SearcherState::Unitialized,
            results: vec![],
            selected: 0,
            theme,
        }
    }
}

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

impl State {
    fn view(&self) -> widget::Column<Message> {
        const TEXT_SIZE: f32 = SIZE_MEDIUM;
        const ICON_SIZE: f32 = TEXT_SIZE * 1.2;
        let search_bar = widget::text_input("Search...", &self.input)
            .on_input(Message::ContentChanged)
            .on_submit(Message::OpenSelected)
            .size(SIZE_MEDIUM)
            .id("searchbar");
        let results = iced::widget::scrollable(
            iced::widget::column(
                self.results
                    .iter()
                    .enumerate()
                    .map(|(i, result)| {
                        let icon: Element<Message> = match result.icon() {
                            searcher::Icon::Svg(path) => widget::svg(path)
                                .width(ICON_SIZE)
                                .height(ICON_SIZE)
                                //.style(|_, _| widget::svg::Style {
                                //color: Some(self.theme.palette().background),
                                //})
                                .into(),
                            searcher::Icon::Raster(path) => widget::image(path)
                                .content_fit(iced::ContentFit::ScaleDown)
                                .width(ICON_SIZE)
                                .height(ICON_SIZE)
                                .into(),
                            searcher::Icon::None => widget::Space::new(ICON_SIZE, ICON_SIZE).into(),
                        };
                        //let icon = widget::Container::new(icon)
                        //.style(move |_| {
                        //widget::container::background(self.theme.palette().background)
                        //})
                        //.height(iced::Length::Shrink)
                        //.width(iced::Length::Shrink);
                        let name = iced::widget::text(result.to_string()).size(TEXT_SIZE);
                        let row = widget::row![icon, name]
                            .height(iced::Length::Shrink)
                            .width(iced::Length::Fill)
                            .padding([0.0, SIZE_SMALL])
                            .spacing(SIZE_SMALL)
                            .wrap();

                        let background = if i == self.selected {
                            self.theme.palette().primary
                        } else {
                            self.theme.palette().background
                        };
                        let container = widget::Container::new(row)
                            .style(move |_| widget::container::background(background));
                        container
                    })
                    .map(Element::from),
            )
            .spacing(10),
        )
        .height(iced::Fill);
        widget::column![search_bar, results]
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::ContentChanged(content) => {
                self.input = content.clone();
                match &mut self.searcher {
                    SearcherState::Initialized(searcher) => {
                        searcher.send(searcher::Message::UpdatePattern(content));
                    }
                    SearcherState::Unitialized => {}
                }
                iced::Task::none()
            }
            Message::Searcher(event) => {
                match event {
                    searcher::Event::Initialized((searcher, injector)) => {
                        self.searcher = SearcherState::Initialized(searcher);
                        // TODO: Not this
                        inject_entries(injector.clone());
                        inject_paths(injector);
                    }
                    searcher::Event::FoundResults(results) => {
                        self.results = results;
                    }
                }
                iced::Task::none()
            }
            Message::RequestClose => iced::exit(),
            Message::SelectUp => {
                self.selected = self.selected.saturating_sub(1);
                iced::Task::none()
            }
            Message::SelectDown => {
                self.selected = std::cmp::min(self.selected + 1, self.results.len() - 1);
                iced::Task::none()
            }
            Message::OpenSelected => {
                println!("Opening {}!", self.selected);
                self.results[self.selected]
                    .open()
                    .expect("Error opening option");
                iced::Task::none()
            }
            Message::TaskWindowOpen(id) => {
                println!("Window {} opening", id);
                iced::Task::none()
            }
            Message::TaskWindowClose => {
                println!("Window closing");
                iced::Task::none()
            }
        }
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

    fn theme(&self) -> iced::Theme {
        self.theme.clone()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    iced::application("A cool counter", State::update, State::view)
        .theme(State::theme)
        .subscription(State::subscription)
        .run_with(|| (State::default(), widget::text_input::focus("searchbar")))?;

    Ok(())
}
