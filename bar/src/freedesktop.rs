use std::path::PathBuf;

use image::RgbaImage;
use log::{trace, warn};
use rand::Rng;
use rand::distributions::Alphanumeric;
use system_tray::item::IconPixmap;

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

pub fn tmp_image_from_data(image_data: &IconPixmap) -> Option<PathBuf> {
    // Generate a path in the /tmp directory
    let path = tmp_path()?;

    // Create and save the image
    let Some(image) = RgbaImage::from_raw(
        image_data.width as u32,
        image_data.height as u32,
        image_data.pixels.clone(),
    ) else {
        warn!("Failed to create RGBA image");
        return None;
    };
    let save_result = image.save(&path);

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

/// Gets a path for an icon by first checking if the passed icon is a path that
/// exists, and if not, searches for a matching freedesktop icon.
pub fn find_icon_path(icon_name_or_path: &str) -> Option<PathBuf> {
    /// Freedesktop Icon Theme name
    const THEME: &str = "Gruvbox-Plus-Dark";

    trace!("Checking path {icon_name_or_path}");
    // Paths are supposed to be prepended with "file://" but in practice many are not
    let path: PathBuf = icon_name_or_path.replace("file://", "").into();
    if path.exists() {
        return Some(path);
    }

    let icon = freedesktop_icons::lookup(icon_name_or_path)
        .with_context("Status")
        .with_cache()
        .force_svg()
        .with_theme(THEME)
        .find()
        .or(freedesktop_icons::lookup(icon_name_or_path)
            .with_cache()
            .force_svg()
            .with_theme(THEME)
            .find());

    match &icon {
        Some(path) => trace!("Found icon {} at {}", icon_name_or_path, path.display()),
        None => trace!("Unable to find icon {}", icon_name_or_path),
    }

    icon
}
