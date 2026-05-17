use crate::effect::{EffectType, apply_effect};
use std::path::PathBuf;
use std::process::Command;
use xcap::Monitor;
use xcap::image::{RgbaImage, imageops};

pub struct ScreenshotDisplay {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) image: RgbaImage,
}

/// Get the lock screen image and apply an effect
/// `sigma` and `radius` are used in the Gaussian blur to affect the strength.
pub fn get_screenshots(effect: EffectType, sigma: f32, radius: f32) -> Vec<ScreenshotDisplay> {
    let screens: Vec<Monitor> = Monitor::all().unwrap();
    let mut displays: Vec<ScreenshotDisplay> = Vec::new();

    for screen in screens {
        let image = screen.capture_image().unwrap();
        displays.push(ScreenshotDisplay {
            x: screen.x().unwrap_or(0),
            y: screen.y().unwrap_or(0),
            image: apply_effect(&image, sigma, radius, effect),
        });
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

/// Saves the composite image to a temporary path
pub fn save_composite(image: &RgbaImage) -> PathBuf {
    let path = PathBuf::from("/tmp/lockscreen.png");
    image.save(&path).unwrap();
    path
}

/// Executes `i3lock` with the corresponding image to show for each of the lock screens
pub fn lock_screen(image_path: &PathBuf) {
    Command::new("i3lock")
        .arg("-i")
        .arg(image_path)
        .status()
        .expect("failed to execute i3lock");
}
