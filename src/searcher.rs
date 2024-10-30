use derive_more::Debug;
use iced::futures::channel::mpsc;
use iced::futures::{select, SinkExt, Stream, StreamExt};

pub fn nucleo() -> impl Stream<Item = Event> {
    iced::stream::channel(100, |mut output| async move {
        // Create a new Nucleo worker
        let (notify_sender, mut notifier) = mpsc::channel(100);
        let mut notify_on_patterns = notify_sender.clone();
        let notify = std::sync::Arc::new(move || {
            let _ = iced::futures::executor::block_on(notify_sender.clone().send(()));
        });
        let mut nucleo: nucleo::Nucleo<String> =
            nucleo::Nucleo::new(nucleo::Config::DEFAULT, notify, None, 1);

        // Create the channel to communicate with the GUI
        let (sender, mut receiver) = mpsc::channel(100);

        // Let the GUI know that the searcher is initialized
        let _ = output
            .send(Event::Initialized((Searcher(sender), nucleo.injector())))
            .await;

        loop {
            select! {
                message = receiver.select_next_some() => {
                    println!("Received {:?}", message);
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
                    println!("Searching...");
                    let status = nucleo.tick(10);

                    if status.changed {
                        let snapshot = nucleo.snapshot();
                        let range = 0..snapshot.matched_item_count();
                        let results: Vec<String> = snapshot
                            .matched_items(range)
                            .map(|item| format!("{:?}", item.matcher_columns))
                            .collect();

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
    Initialized((Searcher, nucleo::Injector<String>)),
    FoundResults(Vec<String>),
    Testing(String),
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
