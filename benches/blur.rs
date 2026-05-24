use divan::{Bencher, bench, black_box};

use libblur::{
    BlurImage, BlurImageMut, ConvolutionMode, EdgeMode, EdgeMode2D, FastBlurChannels,
    GaussianBlurParams, ThreadingPolicy, gaussian_blur,
};
use lockscreen::effect::{
    rgba_image_from_box_gaussian_fast, rgba_image_from_gaussian_asymmetric,
    rgba_image_from_gaussian_blur, rgba_image_from_hoz_box_gaussian_blur, rgba_image_from_pixelate,
};
use xcap::image::{Rgba, RgbaImage};

fn main() {
    divan::main();
}

fn test_image() -> RgbaImage {
    RgbaImage::from_fn(1920, 1080, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
    })
}

/// Box-blur approximation. sigma=10, radius=10 -> derives passes = round(3*100/(10*11)) = 3.
#[bench]
fn lockscreen_gaussian_box_blur(bencher: Bencher) {
    let img = test_image();
    bencher.bench_local(|| rgba_image_from_hoz_box_gaussian_blur(black_box(&img), 10.0, 10.0));
}

/// Gaussian blur from box-blur approximation
#[bench]
fn lockscreen_gaussian(bencher: Bencher) {
    let img = test_image();
    bencher.bench_local(|| rgba_image_from_gaussian_blur(black_box(&img), 10.0));
}

/// Gaussian asymmetric blur from box-blur approximation
#[bench]
fn lockscreen_gaussian_asymmetric(bencher: Bencher) {
    let img = test_image();
    bencher.bench_local(|| rgba_image_from_gaussian_asymmetric(black_box(&img), 10.0, 10.0));
}

/// Fast mode: downsample -> low-res box blur -> upscale. Lower fidelity, much cheaper.
#[bench]
fn lockscreen_gaussian_fast(bencher: Bencher) {
    let img = test_image();
    bencher.bench_local(|| rgba_image_from_box_gaussian_fast(black_box(&img), 10.0, 10.0));
}

/// libblur true analytical Gaussian (O(R), SIMD), single-threaded — matches our single-thread box blur.
#[bench]
fn libblur_gaussian_st(bencher: Bencher) {
    let img = test_image();
    let raw = img.as_raw().clone();
    let (w, h) = (img.width(), img.height());
    bencher.bench_local(|| {
        let src = BlurImage::borrow(&raw, w, h, FastBlurChannels::Channels4);
        let mut dst = BlurImageMut::alloc(w, h, FastBlurChannels::Channels4);
        gaussian_blur(
            black_box(&src),
            &mut dst,
            GaussianBlurParams::new_from_sigma(10.0),
            EdgeMode2D::new(EdgeMode::Clamp),
            ThreadingPolicy::Single,
            ConvolutionMode::FixedPoint,
        )
        .unwrap();
        dst
    });
}

/// Same blur, but libblur's adaptive multithreading — its intended real-world mode.
#[bench]
fn libblur_gaussian_mt(bencher: Bencher) {
    let img = test_image();
    let raw = img.as_raw().clone();
    let (w, h) = (img.width(), img.height());
    bencher.bench_local(|| {
        let src = BlurImage::borrow(&raw, w, h, FastBlurChannels::Channels4);
        let mut dst = BlurImageMut::alloc(w, h, FastBlurChannels::Channels4);
        gaussian_blur(
            black_box(&src),
            &mut dst,
            GaussianBlurParams::new_from_sigma(10.0),
            EdgeMode2D::new(EdgeMode::Clamp),
            ThreadingPolicy::Adaptive,
            ConvolutionMode::FixedPoint,
        )
        .unwrap();
        dst
    });
}

/// fastblur -- the original blur backend (3 box passes, f32, single-threaded). Each iter pays the
/// RGBA -> RGB extract fastblur's `Vec<[u8;3]>` API forces; RGB -> RGBA repack omitted.
#[bench]
fn fastblur_gaussian(bencher: Bencher) {
    let img = test_image();
    let w = img.width() as usize;
    let h = img.height() as usize;
    let raw = img.as_raw().clone();
    bencher.bench_local(|| {
        let mut px: Vec<[u8; 3]> = raw.chunks_exact(4).map(|c| [c[0], c[1], c[2]]).collect();
        fastblur::gaussian_blur(black_box(&mut px), w, h, 10.0);
        px
    });
}

#[bench]
fn lockscreen_pixelate(bencher: Bencher) {
    let img = test_image();
    bencher.bench_local(|| rgba_image_from_pixelate(&img, 16))
}
