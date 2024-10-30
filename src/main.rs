use derive_more::Debug;
use freedesktop_desktop_entry::DesktopEntry;
use iced::{widget::column, widget::text_input, Element, Subscription};

mod searcher;

fn get_desktop_entries() -> Vec<DesktopEntry<'static>> {
    let locales = freedesktop_desktop_entry::get_languages_from_env();
    freedesktop_desktop_entry::Iter::new(freedesktop_desktop_entry::default_paths())
        .entries(Some(&locales))
        .filter(|entry| !entry.no_display())
        .collect()
}

fn inject_entries(injector: nucleo::Injector<String>) {
    let locales = freedesktop_desktop_entry::get_languages_from_env();
    get_desktop_entries().into_iter().for_each(|entry| {
        let name = entry.name(&locales).unwrap_or_default().into_owned();
        let comment = entry.comment(&locales).unwrap_or_default().into_owned();
        let generic_name = entry
            .generic_name(&locales)
            .unwrap_or_default()
            .into_owned();
        let info = format!("{name} {comment} {generic_name}");

        injector.push(info, |info, cols| {
            cols[0] = info.clone().into();
        });
    });
}

fn inject_paths(injector: nucleo::Injector<String>) {
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
            let info = format!("{}", entry.path().display());
            injector.push(info, |info, cols| {
                cols[0] = info.clone().into();
            });
        });
    println!("injected {} paths!", injector.injected_items());
}

struct State {
    input: String,
    searcher: SearcherState,
    results: Vec<String>,
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
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    ContentChanged(String),
    Searcher(searcher::Event),
}

impl State {
    fn view(&self) -> iced::widget::Column<Message> {
        let search_bar = text_input("Search...", &self.input)
            .on_input(Message::ContentChanged)
            .id("searchbar");
        let results = iced::widget::scrollable(
            iced::widget::column(
                self.results
                    .iter()
                    .map(iced::widget::text)
                    .map(Element::from),
            )
            .spacing(10),
        )
        .height(iced::Fill);
        column![search_bar, results]
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ContentChanged(content) => {
                self.input = content.clone();
                match &mut self.searcher {
                    SearcherState::Initialized(searcher) => {
                        searcher.send(searcher::Message::UpdatePattern(content));
                    }
                    SearcherState::Unitialized => {}
                }
            }
            Message::Searcher(event) => match event {
                searcher::Event::Initialized((searcher, injector)) => {
                    self.searcher = SearcherState::Initialized(searcher);
                    // TODO: Not this
                    //inject_entries(injector.clone());
                    inject_paths(injector);
                }
                searcher::Event::FoundResults(results) => {
                    self.results = results;
                }
                searcher::Event::Testing(msg) => {
                    println!("Testing: {msg}");
                }
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(searcher::nucleo).map(Message::Searcher)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let count = walkdir::WalkDir::new("/home/baelyk/")
    //.into_iter()
    //.filter_map(|entry| {
    //let Ok(entry) = entry else {
    //return None;
    //};
    //if entry.file_type().is_file() {
    //return Some(entry);
    //}
    //return None;
    //})
    //.count();
    //println!("Found {count} files");
    iced::application("A cool counter", State::update, State::view)
        .subscription(State::subscription)
        .run_with(|| (State::default(), text_input::focus("searchbar")))?;

    Ok(())
}
