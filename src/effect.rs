use fastblur::gaussian_blur;
use fastblur::gaussian_blur_asymmetric;
use screenshots::image::{Rgba, RgbaImage};

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub(crate) enum EffectType {
    Gaussian,
    GaussianAsymmetric,
    Pixelate,
}

pub(crate) struct ImageValues {
    pixels: Vec<[u8; 3]>,
    alphas: Vec<u8>,
    width: u32,
    height: u32,
}

/// Converts an `RgbaImage` to its raw pixel values
/// stored with a fixed array size represented as [R, G, B]
fn image_to_pixels(image: &RgbaImage) -> Vec<[u8; 3]> {
    image.pixels().map(|p| [p[0], p[1], p[2]]).collect()
}

/// Converts an `RgbaImage` to its raw alpha values.
fn image_to_alphas(image: &RgbaImage) -> Vec<u8> {
    image.pixels().map(|p| p[3]).collect()
}

/// Converts `ImageValues` to new `RgbaImage` [R, G, B, A]
/// We only have pixel values available and need to produce a new `RgbaImage`
fn rgba_image_from_values(value: &ImageValues) -> RgbaImage {
    RgbaImage::from_fn(value.width, value.height, |x, y| {
        let idx = y as usize * value.width as usize + x as usize;

        Rgba([
            value.pixels[idx][0],
            value.pixels[idx][1],
            value.pixels[idx][2],
            value.alphas[idx],
        ])
    })
}

/// Create a new `RgbaImage` with an applied Gaussian blur effect.
/// This will give a smooth blur effect across the image.
pub(crate) fn rgba_image_from_gaussian_blur(image: &RgbaImage, sigma: f32) -> RgbaImage {
    let mut pixels = image_to_pixels(image);
    gaussian_blur(
        &mut pixels,
        image.width() as usize,
        image.height() as usize,
        sigma,
    );
    rgba_image_from_values(&ImageValues {
        pixels,
        alphas: image_to_alphas(image),
        width: image.width(),
        height: image.height(),
    })
}

/// Create a new `RgbaImage` with an Asymmetrical Gaussian blur.
/// This will give a blur effect that appears more stretched.
pub(crate) fn rgba_image_from_gaussian_asymmetric(
    image: &RgbaImage,
    sigma: f32,
    radius: f32,
) -> RgbaImage {
    let mut pixels = image_to_pixels(image);
    gaussian_blur_asymmetric(
        &mut pixels,
        image.width() as usize,
        image.height() as usize,
        sigma,
        radius,
    );
    rgba_image_from_values(&ImageValues {
        pixels,
        alphas: image_to_alphas(image),
        width: image.width(),
        height: image.height(),
    })
}

/// Create a new `RgbaImage` that has been pixelated.
///
/// 1. Clamp the block to the image bounds
/// 2. Sum all the Rgba channels across the pixel blocks
/// 3. Average each pixel given the block width and height (bw * bh)
/// 4. Write each pixel average out then advance to the next block
///
/// Each block becomes a single flat color where a larger block size produces coarser pixelation.
///
/// `block_size` (sigma) of one will lead to a completely normal
/// image since the average of one pixel is itself negating any pixelation.
pub(crate) fn rgba_image_from_pixelate(image: &RgbaImage, block_size: u32) -> RgbaImage {
    let block_size = block_size.max(1);

    let width = image.width();
    let height = image.height();
    let mut output = image.clone();

    let mut y = 0;
    while y < height {
        let mut x = 0;

        while x < width {
            // Clamp block width and block height within the bounds of the image
            let block_width = block_size.min(width - x);
            let block_height = block_size.min(height - y);

            let count = (block_width * block_height) as u64;

            let (mut r, mut g, mut b, mut a) = (0u64, 0u64, 0u64, 0u64);
            for pixel_y in y..y + block_height {
                for pixel_x in x..x + block_width {
                    let p = image.get_pixel(pixel_x, pixel_y);
                    r += p[0] as u64;
                    g += p[1] as u64;
                    b += p[2] as u64;
                    a += p[3] as u64;
                }
            }

            // Average each pixel by count and reconstruct the output based on the input
            let avg = Rgba([
                (r / count) as u8,
                (g / count) as u8,
                (b / count) as u8,
                (a / count) as u8,
            ]);
            for pixel_y in y..y + block_height {
                for pixel_x in x..x + block_width {
                    output.put_pixel(pixel_x, pixel_y, avg);
                }
            }

            x += block_size;
        }

        y += block_size;
    }

    output
}

/// Apply one of the available `EffectType`(s) to the `RgbaImage`.
///
/// `sigma` is the standard deviation.
///
/// `radius` is distance of which the blur effect is spread across.
pub(crate) fn apply_effect(
    image: &RgbaImage,
    sigma: f32,
    radius: f32,
    effect: EffectType,
) -> RgbaImage {
    match effect {
        EffectType::Gaussian => rgba_image_from_gaussian_blur(image, sigma),
        EffectType::GaussianAsymmetric => rgba_image_from_gaussian_asymmetric(image, sigma, radius),
        EffectType::Pixelate => rgba_image_from_pixelate(image, sigma as u32),
    }
}
