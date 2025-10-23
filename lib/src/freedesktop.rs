use std::path::PathBuf;

use freedesktop_desktop_entry::{Iter, default_paths, get_languages_from_env};
use image::{RgbImage, RgbaImage};
use log::{debug, trace, warn};
use rand::Rng;
use rand::distributions::Alphanumeric;

pub fn find_app_name(desktop_entry_name: &str) -> Option<String> {
    let locales = get_languages_from_env();
    let mut entries = Iter::new(default_paths()).entries(Some(&locales));

    let desktop_entry_name = desktop_entry_name.to_lowercase();
    if let Some(desktop_entry) =
        entries.find(|desktop_entry| desktop_entry.appid.to_lowercase() == desktop_entry_name)
    {
        if let Some(name) = desktop_entry.name(&locales) {
            return Some(name.into_owned());
        } else {
            debug!("No name found for {}", desktop_entry_name);
        }
    } else {
        debug!("No desktop entry found for {}", desktop_entry_name);
    }

    None
}

fn tmp_path() -> Option<PathBuf> {
    let mut tries = 0;
    while tries < 3 {
        tries += 1;

        let filename: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        let path = PathBuf::from(format!("/tmp/{}.png", filename));

        if path.try_exists().is_ok_and(|exists| !exists) {
            return Some(path);
        }
    }

    warn!("Unable to generate a temporary path");
    None
}

pub fn tmp_image_from_data(width: u32, height: u32, data: Vec<u8>, alpha: bool) -> Option<PathBuf> {
    // Generate a path in the /tmp directory
    let path = tmp_path()?;

    // Create and save the image
    let save_result = if alpha {
        let Some(image) = RgbaImage::from_raw(width, height, data) else {
            warn!("Failed to create RGBA image");
            return None;
        };
        image.save(&path)
    } else {
        let Some(image) = RgbImage::from_raw(width, height, data) else {
            warn!("Failed to create RGB image");
            return None;
        };
        image.save(&path)
    };

    if let Err(err) = save_result {
        warn!(
            "Failed to save image to {} with error {}",
            path.display(),
            err
        );
        return None;
    };

    Some(path)
}

/// Freedesktop Icon Theme name
const ICON_THEME: &str = "Gruvbox-Plus-Dark";

/// Gets a path for an icon by first checking if the passed icon is a path that
/// exists, and if not, searches for a matching freedesktop icon.
pub fn find_icon_path(icon_name_or_path: &str, context: Option<&str>) -> Option<PathBuf> {
    trace!("Checking path {icon_name_or_path}");
    // Paths are supposed to be prepended with "file://" but in practice many are not
    let path: PathBuf = icon_name_or_path.replace("file://", "").into();
    if path.exists() {
        return Some(path);
    }

    let finder = freedesktop_icons::lookup(icon_name_or_path)
        .with_cache()
        .force_svg()
        .with_theme(ICON_THEME);

    let finder = if let Some(context) = context {
        finder.with_context(context)
    } else {
        finder
    };

    let icon = finder.find();

    match &icon {
        Some(path) => trace!("Found icon {} at {}", icon_name_or_path, path.display()),
        None => trace!("Unable to find icon {}", icon_name_or_path),
    }

    icon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_default_notifications_icon() {
        let found_icon = find_icon_path("notifications", None).expect("Should find icon");
        let path = PathBuf::from(ICON_THEME.to_owned() + "/actions/24/notifications.svg");
        assert!(dbg!(found_icon).ends_with(path));
    }
}
