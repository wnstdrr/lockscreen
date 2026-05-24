use crate::effect::{EffectType, apply_effect};
use std::path::PathBuf;
use xcap::Monitor;
use xcap::image::{ImageError, RgbaImage, imageops};

pub struct ScreenshotDisplay {
    pub x: i32,
    pub y: i32,
    pub image: RgbaImage,
}

/// Get the lock screen image and apply an effect
/// `sigma` and `radius` are used in the Gaussian blur to affect the strength.
pub fn get_screenshots(
    effect: EffectType,
    sigma: f32,
    radius: f32,
    fast: bool,
) -> Vec<ScreenshotDisplay> {
    let screens = Monitor::all().unwrap_or_default();
    let mut displays = Vec::new();

    for screen in screens {
        match screen.capture_image() {
            Ok(image) => displays.push(ScreenshotDisplay {
                x: screen.x().unwrap_or(0),
                y: screen.y().unwrap_or(0),
                image: apply_effect(&image, sigma, radius, effect, fast),
            }),

            Err(_) => {
                // We could not get this display for one reason or another.
                // Skip adding it to the vector of displays.
                continue;
            }
        }
    }

    displays
}

/// Composes screenshot displays into a single image overlay
pub fn compose_displays(screenshots: &[ScreenshotDisplay]) -> RgbaImage {
    if screenshots.len() == 1 {
        return screenshots[0].image.clone();
    }

    // Get min x,y values from the screenshot display
    let min_x = screenshots.iter().map(|s| s.x).min().unwrap_or(0);
    let min_y = screenshots.iter().map(|s| s.y).min().unwrap_or(0);

    // Get max x,y values from the screenshot display
    let max_x = screenshots
        .iter()
        .map(|s| s.x + s.image.width() as i32)
        .max()
        .unwrap_or(0);

    let max_y = screenshots
        .iter()
        .map(|s| s.y + s.image.height() as i32)
        .max()
        .unwrap_or(0);

    let width = (max_x - min_x) as u32;
    let height = (max_y - min_y) as u32;

    let mut composite = RgbaImage::new(width, height);

    // Overlay each of the screens as the x,y coordinates
    for screen in screenshots {
        let x = (screen.x - min_x) as i64;
        let y = (screen.y - min_y) as i64;
        imageops::overlay(&mut composite, &screen.image, x, y);
    }

    composite
}

/// Saves the composite image to tmpfs
pub fn save_composite(image: &RgbaImage) -> Result<PathBuf, ImageError> {
    let path = PathBuf::from("/dev/shm/lockscreen.png");
    image.save(&path)?;

    Ok(path)
}
