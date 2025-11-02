use std::sync::Arc;

use derive_more::Debug;
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt, select};
use log::{debug, trace};

use crate::providers::Entry;

pub fn nucleo() -> impl Stream<Item = Event> {
    iced::stream::channel(100, async move |mut output| {
        // Create a new Nucleo worker
        let (notify_sender, mut notifier) = mpsc::channel(1_000_000_000);
        let mut notify_on_patterns = notify_sender.clone();
        let notify = std::sync::Arc::new(move || {
            let _ = iced::futures::executor::block_on(notify_sender.clone().send(()));
        });
        let mut nucleo: nucleo::Nucleo<Arc<dyn Entry>> =
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
                    debug!("Received {:?}", message);
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
                    debug!("Searching...");
                    let status = nucleo.tick(5);
                    trace!("Status: {status:#?}");

                    if status.changed {
                        let snapshot = nucleo.snapshot();
                        debug!("Found {} results", snapshot.matched_item_count());
                        let items = std::cmp::min(100, snapshot.matched_item_count());
                        let range = 0..items;
                        let results: Vec<Arc<dyn Entry>> = snapshot
                            .matched_items(range)
                            .map(|item| item.data.clone())
                            .collect();

                        debug!("Sending {} results", results.len());
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
    Initialized((Searcher, nucleo::Injector<Arc<dyn Entry>>)),
    FoundResults(Vec<Arc<dyn Entry>>),
}

#[derive(Debug, Clone)]
pub struct Searcher(mpsc::Sender<Message>);
impl Searcher {
    pub fn send(&mut self, message: Message) {
        debug!("Sending a message");
        self.0
            .try_send(message)
            .expect("Unable to send message to Searcher");
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdatePattern(String),
}
