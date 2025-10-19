use std::{collections::HashMap, path::PathBuf};

use iced::futures::{SinkExt, Stream};
use log::{debug, warn};
use system_tray::{
    client::{Client, Event},
    item::StatusNotifierItem,
};

pub use system_tray::item::Status;

use crate::freedesktop::{find_icon_path, tmp_image_from_data};

pub fn tray() -> impl Stream<Item = TrayMessage> {
    iced::stream::channel(100, async move |mut output| {
        let client = Client::new().await.unwrap();
        let mut tray_rx = client.subscribe();

        let initial_items: TrayItems = client
            .items()
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(destination, (item, _))| (destination, item.into()))
            .collect();

        output
            .send(TrayMessage::Initialized(initial_items))
            .await
            .expect("Unable to send initial items");

        while let Ok(event) = tray_rx.recv().await {
            debug!("Event: {:#?}", event);
            match event {
                Event::Add(destination, item) => {
                    let item: TrayItem = (*item).into();
                    output
                        .send(TrayMessage::Add(destination, item))
                        .await
                        .unwrap();
                }
                Event::Update(destination, update) => {
                    debug!("Update {}: {:#?}", destination, update);
                }
                Event::Remove(destination) => {
                    output.send(TrayMessage::Remove(destination)).await.unwrap();
                }
            }
        }
    })
}

pub type TrayItems = HashMap<String, TrayItem>;

#[derive(Debug, Clone)]
pub enum TrayMessage {
    Initialized(TrayItems),
    Add(String, TrayItem),
    Remove(String),
}

#[derive(Debug, Clone)]
pub struct TrayItem {
    pub title: String,
    pub status: Status,
    pub icon: PathBuf,
}

impl From<StatusNotifierItem> for TrayItem {
    fn from(item: StatusNotifierItem) -> Self {
        let title = item.title.unwrap_or(item.id);
        let status = item.status;
        let icon = item
            .icon_name
            .and_then(|icon_name| find_icon_path(&icon_name))
            .or_else(|| {
                if let Some(pixmap) = item.icon_pixmap {
                    tmp_image_from_data(&pixmap[0])
                } else {
                    None
                }
            })
            .or_else(|| find_icon_path("notifications"))
            .expect("Unable to find default icon");

        Self {
            title,
            status,
            icon,
        }
    }
}

enum MenuItem {
    Seperator,
    Item {
        id: i32,
        label: String,
        submenu: Vec<MenuItem>,
    },
}

impl From<system_tray::menu::MenuItem> for MenuItem {
    fn from(item: system_tray::menu::MenuItem) -> Self {
        match item.menu_type {
            system_tray::menu::MenuType::Separator => Self::Seperator,
            system_tray::menu::MenuType::Standard => Self::Item {
                id: item.id,
                label: item.label.unwrap_or_default(),
                submenu: item.submenu.into_iter().map(|item| item.into()).collect(),
            },
        }
    }
}
