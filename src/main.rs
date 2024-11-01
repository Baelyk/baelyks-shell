use derive_more::Debug;
use freedesktop_desktop_entry::DesktopEntry;
use iced::{
    keyboard::{key, on_key_press, on_key_release},
    widget::{column, text_input},
    window, Element, Subscription,
};
use searcher::SearchItem;

mod searcher;

lazy_static::lazy_static! {
    pub static ref LOCALES: Vec<String> = freedesktop_desktop_entry::get_languages_from_env();
}

fn get_desktop_entries() -> Vec<DesktopEntry<'static>> {
    freedesktop_desktop_entry::Iter::new(freedesktop_desktop_entry::default_paths())
        .entries(Some(&LOCALES))
        .filter(|entry| !entry.no_display())
        .collect()
}

fn inject_entries(injector: nucleo::Injector<SearchItem>) {
    get_desktop_entries().into_iter().for_each(|entry| {
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
    window_id: Option<window::Id>,
}

enum SearcherState {
    Unitialized,
    Initialized(searcher::Searcher),
}

impl Default for State {
    fn default() -> Self {
        State {
            input: String::new(),
            searcher: SearcherState::Unitialized,
            results: vec![],
            selected: 0,
            window_id: None,
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

impl State {
    fn view(&self, _: window::Id) -> iced::widget::Column<Message> {
        let search_bar = text_input("Search...", &self.input)
            .on_input(Message::ContentChanged)
            .on_submit(Message::OpenSelected)
            .id("searchbar");
        let results = iced::widget::scrollable(
            iced::widget::column(
                self.results
                    .iter()
                    .enumerate()
                    .map(|(i, result)| {
                        let color = if i == self.selected {
                            iced::color!(0xff0000)
                        } else {
                            iced::color!(0x000000)
                        };
                        iced::widget::text(result.to_string()).color(color)
                    })
                    .map(Element::from),
            )
            .spacing(10),
        )
        .height(iced::Fill);
        column![search_bar, results]
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
            Message::RequestClose => match self.window_id {
                Some(_id) => {
                    // TODO: Just close window, don't exit
                    iced::exit()
                }
                None => iced::Task::none(),
            },
            Message::SelectUp => {
                self.selected = self.selected.saturating_sub(1);
                iced::Task::none()
            }
            Message::SelectDown => {
                self.selected = std::cmp::min(self.selected + 1, self.results.len());
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    iced::daemon("A cool counter", State::update, State::view)
        .subscription(State::subscription)
        .run_with(|| {
            let mut state = State::default();
            let (id, window_task) = window::open(window::Settings::default());
            state.window_id = Some(id);
            let tasks = [
                window_task.map(Message::TaskWindowOpen),
                text_input::focus("searchbar"),
            ];
            (state, iced::Task::batch(tasks))
        })?;

    Ok(())
}
