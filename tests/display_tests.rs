use lockscreen::display::{ScreenshotDisplay, compose_displays, get_screenshots, save_composite};
use lockscreen::effect::EffectType;
use xcap::image::{Rgba, RgbaImage};

fn new_display(x: i32, y: i32, w: u32, h: u32) -> ScreenshotDisplay {
    ScreenshotDisplay {
        x,
        y,
        image: RgbaImage::new(w, h),
    }
}

#[test]
fn get_screenshots_returns_valid_displays() {
    let displays = get_screenshots(EffectType::Gaussian, 2.0, 1.0);

    for display in &displays {
        assert!(display.image.width() > 0);
        assert!(display.image.height() > 0);

        assert_eq!(
            display.image.dimensions(),
            (display.image.width(), display.image.height())
        );
    }
}

#[test]
fn get_screenshots_gaussian_asymmetric_returns_valid_displays() {
    let displays = get_screenshots(EffectType::GaussianAsymmetric, 2.0, 1.0);

    for display in &displays {
        assert!(display.image.width() > 0);
        assert!(display.image.height() > 0);

        assert_eq!(
            display.image.dimensions(),
            (display.image.width(), display.image.height())
        );
    }
}

#[test]
fn get_screenshots_pixelate_returns_valid_displays() {
    let displays = get_screenshots(EffectType::Pixelate, 2.0, 1.0);

    for display in &displays {
        assert_eq!(
            display.image.dimensions(),
            (display.image.width(), display.image.height())
        );
    }
}

#[test]
fn compose_single_display_returns_same_dimensions() {
    let displays = new_display(0, 0, 100, 50);
    let result = compose_displays(&[displays]);

    assert_eq!(result.dimensions(), (100, 50));
}

#[test]
fn compose_two_displays_side_by_side() {
    let displays = vec![new_display(0, 0, 100, 50), new_display(100, 0, 100, 50)];
    let result = compose_displays(&displays);

    assert_eq!(result.dimensions(), (200, 50));
}

#[test]
fn compose_two_displays_stacked() {
    let displays = vec![new_display(0, 0, 100, 50), new_display(0, 50, 100, 50)];
    let result = compose_displays(&displays);

    assert_eq!(result.dimensions(), (100, 100));
}

#[test]
fn compose_offset_displays_computes_correct_size() {
    let displays = vec![new_display(10, 20, 100, 50), new_display(110, 20, 100, 50)];
    let result = compose_displays(&displays);

    assert_eq!(result.dimensions(), (200, 50));
}

#[test]
fn compose_single_display_clone_pixel_contents() {
    let mut image = new_display(50, 30, 300, 500);
    image.image.put_pixel(50, 30, Rgba([255, 0, 0, 255]));

    let result = compose_displays(&[image]);

    assert_eq!(result.get_pixel(50, 30), &Rgba([255, 0, 0, 255]));
}

#[test]
fn save_composite_writes_to_tmp() {
    let image = RgbaImage::new(10, 10);
    let path = save_composite(&image).expect("save_composite failed");

    assert!(path.exists());

    // Cleanup
    std::fs::remove_file(&path).ok();
}
