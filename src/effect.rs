use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicUsize, Ordering};
use std::thread;

use clap::ValueEnum;
use xcap::image::{Rgba, RgbaImage};

#[derive(Copy, Clone, ValueEnum)]
pub enum EffectType {
    Gaussian,
    GaussianAsymmetric,
    GaussianBoxBlur,
    Pixelate,
}

/// Clamp a box radius so its sliding window stays in-bounds along an axis of `len` pixels.
/// Required for the unchecked indexing in the kernels (and avoids `len - radius` underflow).
fn clamp_radius(radius: usize, len: usize) -> usize {
    radius.min(len.saturating_sub(1) / 2)
}

/// Pass count to approximate a Gaussian of std dev `sigma` with box radius `r` (per axis).
/// One box pass contributes variance r(r+1)/3; n passes sum to n*r(r+1)/3, so solving for the
/// target variance sigma^2 gives n = 3*sigma^2 / (r(r+1)). Rounded to nearest, floor of one.
fn derive_passes(sigma: f32, r: usize) -> usize {
    if r == 0 {
        return 0;
    }
    let rf = r as f32;
    (3.0 * sigma * sigma / (rf * (rf + 1.0))).round().max(1.0) as usize
}

/// Per-round work descriptor shared with the worker pool. The main thread writes it between
/// barriers; workers read it after the round-start barrier (which supplies the happens-before).
/// Buffer pointers travel as `usize` addresses so the struct stays `Sync` without a `Send` shim.
struct Round {
    phase: AtomicU8, // 0 = horizontal, 1 = vertical
    radius: AtomicUsize,
    recip: AtomicU32,
    half: AtomicU32,
    src: AtomicUsize, // *const u8 as usize (read-only this round)
    dst: AtomicUsize, // *mut u8 as usize (each thread writes only its own region)
    done: AtomicBool,
}

/// Thread count for `work` units of independent work, capped at available parallelism (min 1).
fn thread_count(work: usize) -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(work)
        .max(1)
}

/// Horizontal box blur of a single row. `src`/`dest` are exactly one row (`width * 4` bytes).
/// SAFETY: caller guarantees `2*radius < width`, so every index stays within `[0, width*4)`.
fn blur_row_h(src: &[u8], dest: &mut [u8], width: usize, radius: usize, recip: u32, half: u32) {
    const SHIFT: u32 = 23;

    let mut write_idx = 0usize;
    let mut left_window_idx = 0usize;
    let mut right_window_idx = radius * 4;

    unsafe {
        let first_r = *src.get_unchecked(0) as u32;
        let first_g = *src.get_unchecked(1) as u32;
        let first_b = *src.get_unchecked(2) as u32;
        let first_a = *src.get_unchecked(3) as u32;

        let last_offset = (width - 1) * 4;
        let last_r = *src.get_unchecked(last_offset) as u32;
        let last_g = *src.get_unchecked(last_offset + 1) as u32;
        let last_b = *src.get_unchecked(last_offset + 2) as u32;
        let last_a = *src.get_unchecked(last_offset + 3) as u32;

        let mut sum_r = (radius as u32 + 1) * first_r;
        let mut sum_g = (radius as u32 + 1) * first_g;
        let mut sum_b = (radius as u32 + 1) * first_b;
        let mut sum_a = (radius as u32 + 1) * first_a;

        for j in 0..radius {
            let idx = j * 4;
            sum_r += *src.get_unchecked(idx) as u32;
            sum_g += *src.get_unchecked(idx + 1) as u32;
            sum_b += *src.get_unchecked(idx + 2) as u32;
            sum_a += *src.get_unchecked(idx + 3) as u32;
        }

        for _ in 0..=radius {
            sum_r = sum_r + *src.get_unchecked(right_window_idx) as u32 - first_r;
            sum_g = sum_g + *src.get_unchecked(right_window_idx + 1) as u32 - first_g;
            sum_b = sum_b + *src.get_unchecked(right_window_idx + 2) as u32 - first_b;
            sum_a = sum_a + *src.get_unchecked(right_window_idx + 3) as u32 - first_a;
            right_window_idx += 4;

            *dest.get_unchecked_mut(write_idx) = ((sum_r * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 1) = ((sum_g * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 2) = ((sum_b * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 3) = ((sum_a * recip + half) >> SHIFT) as u8;
            write_idx += 4;
        }

        for _ in (radius + 1)..(width - radius) {
            sum_r = sum_r + *src.get_unchecked(right_window_idx) as u32
                - *src.get_unchecked(left_window_idx) as u32;
            sum_g = sum_g + *src.get_unchecked(right_window_idx + 1) as u32
                - *src.get_unchecked(left_window_idx + 1) as u32;
            sum_b = sum_b + *src.get_unchecked(right_window_idx + 2) as u32
                - *src.get_unchecked(left_window_idx + 2) as u32;
            sum_a = sum_a + *src.get_unchecked(right_window_idx + 3) as u32
                - *src.get_unchecked(left_window_idx + 3) as u32;
            right_window_idx += 4;
            left_window_idx += 4;

            *dest.get_unchecked_mut(write_idx) = ((sum_r * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 1) = ((sum_g * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 2) = ((sum_b * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 3) = ((sum_a * recip + half) >> SHIFT) as u8;
            write_idx += 4;
        }

        for _ in (width - radius)..width {
            sum_r = sum_r + last_r - *src.get_unchecked(left_window_idx) as u32;
            sum_g = sum_g + last_g - *src.get_unchecked(left_window_idx + 1) as u32;
            sum_b = sum_b + last_b - *src.get_unchecked(left_window_idx + 2) as u32;
            sum_a = sum_a + last_a - *src.get_unchecked(left_window_idx + 3) as u32;
            left_window_idx += 4;

            *dest.get_unchecked_mut(write_idx) = ((sum_r * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 1) = ((sum_g * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 2) = ((sum_b * recip + half) >> SHIFT) as u8;
            *dest.get_unchecked_mut(write_idx + 3) = ((sum_a * recip + half) >> SHIFT) as u8;
            write_idx += 4;
        }
    }
}

/// Vertical box blur for the column range `[x0, x0 + sums.len())`, writing through `dst`.
/// `sums` is the per-column scratch for this strip (length == strip width), fully overwritten here.
///
/// SAFETY: `dst` is the dest buffer's base pointer. This writes only columns in `[x0, x0+strip)`,
/// which never overlap a sibling thread's range, so the concurrent writes do not alias. Caller
/// guarantees `2*radius < height`, keeping every src/dst row index within `[0, height*stride)`.
unsafe fn blur_cols_v(
    src: &[u8],
    dst: *mut u8,
    sums: &mut [[u32; 4]],
    x0: usize,
    width: usize,
    height: usize,
    radius: usize,
    recip: u32,
    half: u32,
) {
    const SHIFT: u32 = 23;
    let stride = width * 4;
    let strip_w = sums.len();

    unsafe {
        // Init: (radius+1) copies of first row per column.
        for xi in 0..strip_w {
            let i = (x0 + xi) * 4;
            *sums.get_unchecked_mut(xi) = [
                (radius as u32 + 1) * *src.get_unchecked(i) as u32,
                (radius as u32 + 1) * *src.get_unchecked(i + 1) as u32,
                (radius as u32 + 1) * *src.get_unchecked(i + 2) as u32,
                (radius as u32 + 1) * *src.get_unchecked(i + 3) as u32,
            ];
        }
        // Add real pixels from rows 0..radius.
        for j in 0..radius {
            let row_off = j * stride;
            for xi in 0..strip_w {
                let i = row_off + (x0 + xi) * 4;
                let s = sums.get_unchecked_mut(xi);
                s[0] += *src.get_unchecked(i) as u32;
                s[1] += *src.get_unchecked(i + 1) as u32;
                s[2] += *src.get_unchecked(i + 2) as u32;
                s[3] += *src.get_unchecked(i + 3) as u32;
            }
        }

        for y in 0..height {
            // Incoming bottom-of-window row (clamped at last row for Zone C).
            let add_row = if y + radius < height {
                y + radius
            } else {
                height - 1
            };
            let add_off = add_row * stride;
            // Outgoing top-of-window: first row (Zone A) or real row (Zones B/C).
            let sub_off = if y <= radius {
                0
            } else {
                (y - radius - 1) * stride
            };
            let out_off = y * stride;

            for xi in 0..strip_w {
                let x4 = (x0 + xi) * 4;
                let s = sums.get_unchecked_mut(xi);

                let ai = add_off + x4;
                let si = sub_off + x4;

                s[0] = s[0] + *src.get_unchecked(ai) as u32 - *src.get_unchecked(si) as u32;
                s[1] = s[1] + *src.get_unchecked(ai + 1) as u32 - *src.get_unchecked(si + 1) as u32;
                s[2] = s[2] + *src.get_unchecked(ai + 2) as u32 - *src.get_unchecked(si + 2) as u32;
                s[3] = s[3] + *src.get_unchecked(ai + 3) as u32 - *src.get_unchecked(si + 3) as u32;

                let oi = out_off + x4;
                *dst.add(oi) = ((s[0] * recip + half) >> SHIFT) as u8;
                *dst.add(oi + 1) = ((s[1] * recip + half) >> SHIFT) as u8;
                *dst.add(oi + 2) = ((s[2] * recip + half) >> SHIFT) as u8;
                *dst.add(oi + 3) = ((s[3] * recip + half) >> SHIFT) as u8;
            }
        }
    }
}

/// Run one blur round for this thread's region, reading the shared [`Round`] descriptor.
///
/// SAFETY: `src`/`dst` addresses in `round` point to live buffers for the duration of this round
/// (the surrounding barriers guarantee it). Each thread touches only its own rows (horizontal) or
/// columns (vertical), so concurrent writes never alias and `src` is read-only.
unsafe fn run_round(
    round: &Round,
    strip: &mut [[u32; 4]],
    row0: usize,
    rows_per: usize,
    col0: usize,
    width: usize,
    height: usize,
    stride: usize,
) {
    let radius = round.radius.load(Ordering::Acquire);
    let recip = round.recip.load(Ordering::Acquire);
    let half = round.half.load(Ordering::Acquire);
    let src = round.src.load(Ordering::Acquire) as *const u8;
    let dst = round.dst.load(Ordering::Acquire) as *mut u8;

    if round.phase.load(Ordering::Acquire) == 0 {
        // Horizontal: this thread's band of rows.
        let y_end = (row0 + rows_per).min(height);
        for y in row0..y_end {
            let off = y * stride;
            unsafe {
                let s = std::slice::from_raw_parts(src.add(off), stride);
                let d = std::slice::from_raw_parts_mut(dst.add(off), stride);
                blur_row_h(s, d, width, radius, recip, half);
            }
        }
    } else if !strip.is_empty() {
        // Vertical: this thread's strip of columns.
        unsafe {
            let full_src = std::slice::from_raw_parts(src, height * stride);
            blur_cols_v(
                full_src, dst, strip, col0, width, height, radius, recip, half,
            );
        }
    }
}

/// Worker thread body: park on the start barrier, run the round, park on the end barrier — looping
/// until the main thread sets `done` and releases the start barrier one final time.
fn worker_loop(
    round: &Round,
    barrier: &Barrier,
    strip: &mut [[u32; 4]],
    row0: usize,
    rows_per: usize,
    col0: usize,
    width: usize,
    height: usize,
    stride: usize,
) {
    loop {
        barrier.wait();
        if round.done.load(Ordering::Acquire) {
            break;
        }
        unsafe { run_round(round, strip, row0, rows_per, col0, width, height, stride) };
        barrier.wait();
    }
}

/// Separable box-blur Gaussian approximation with independent per-axis settings.
/// Horizontal and vertical box passes commute (orthogonal linear filters), so all H passes run
/// first, then all V passes. Equal H/V args give a symmetric (isotropic) blur; differing args
/// give a directional/stretched one.
///
/// A single pool of `N` workers (the main thread is worker 0, so only `N-1` are spawned) lives for
/// the whole blur and is driven pass-by-pass through a [`Barrier`], so thread-startup latency is
/// paid once instead of per pass. Buffers ping-pong by swapping which address is `src` vs `dst`.
fn box_blur_gaussian(
    image: &RgbaImage,
    radius_h: usize,
    passes_h: usize,
    radius_v: usize,
    passes_v: usize,
) -> RgbaImage {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let stride = width * 4;

    let mut buf_a = image.as_raw().clone();
    let mut buf_b = vec![0u8; buf_a.len()];
    let mut sums = vec![[0u32; 4]; width];

    let a_addr = buf_a.as_mut_ptr() as usize;
    let b_addr = buf_b.as_mut_ptr() as usize;

    const SHIFT: u32 = 23;
    let recip_h = (1u32 << SHIFT) / (radius_h as u32 * 2 + 1);
    let recip_v = (1u32 << SHIFT) / (radius_v as u32 * 2 + 1);
    let half = 1u32 << (SHIFT - 1);

    let n = thread_count(width.min(height));
    let rows_per = height.div_ceil(n);
    let cols_per = width.div_ceil(n);

    let round = Round {
        phase: AtomicU8::new(0),
        radius: AtomicUsize::new(0),
        recip: AtomicU32::new(0),
        half: AtomicU32::new(half),
        src: AtomicUsize::new(a_addr),
        dst: AtomicUsize::new(b_addr),
        done: AtomicBool::new(false),
    };
    let barrier = Barrier::new(n);

    // Address of the buffer holding the latest result; defaults to the input clone for zero rounds.
    let mut last_dst = a_addr;

    thread::scope(|s| {
        let round = &round;
        let barrier = &barrier;

        // Split the column scratch into n disjoint strips (trailing ones empty when width < n*cols).
        // Strip i covers columns [i*cols_per, ..); main keeps strip 0, workers take 1..n.
        let mut strip_for: Vec<&mut [[u32; 4]]> = Vec::with_capacity(n);
        let mut rest: &mut [[u32; 4]] = &mut sums;
        for _ in 0..n {
            let take = cols_per.min(rest.len());
            let (head, tail) = rest.split_at_mut(take);
            strip_for.push(head);
            rest = tail;
        }
        let mut strip_iter = strip_for.into_iter();
        let main_strip = strip_iter.next().unwrap();
        for id in 1..n {
            let strip = strip_iter.next().unwrap();
            s.spawn(move || {
                worker_loop(
                    round,
                    barrier,
                    strip,
                    id * rows_per,
                    rows_per,
                    id * cols_per,
                    width,
                    height,
                    stride,
                );
            });
        }

        let mut src_addr = a_addr;
        let mut dst_addr = b_addr;

        let mut rounds = Vec::with_capacity(2);
        if radius_h > 0 {
            rounds.extend(std::iter::repeat_n((0u8, radius_h, recip_h), passes_h));
        }
        if radius_v > 0 {
            rounds.extend(std::iter::repeat_n((1u8, radius_v, recip_v), passes_v));
        }

        for (phase, radius, recip) in rounds {
            round.phase.store(phase, Ordering::Release);
            round.radius.store(radius, Ordering::Release);
            round.recip.store(recip, Ordering::Release);
            round.src.store(src_addr, Ordering::Release);
            round.dst.store(dst_addr, Ordering::Release);

            barrier.wait(); // release the pool into this round
            unsafe { run_round(round, main_strip, 0, rows_per, 0, width, height, stride) };
            barrier.wait(); // all workers finished this round

            last_dst = dst_addr;
            std::mem::swap(&mut src_addr, &mut dst_addr);
        }

        // Final release so parked workers observe `done` and exit.
        round.done.store(true, Ordering::Release);
        barrier.wait();
    });

    let result = if last_dst == a_addr { buf_a } else { buf_b };
    RgbaImage::from_raw(width as u32, height as u32, result).unwrap()
}

/// Symmetric box-blur Gaussian. `radius` is the per-pass box radius; `passes` is derived from
/// `sigma` so the result approximates a true Gaussian of that std dev.
pub fn rgba_image_from_hoz_box_gaussian_blur(
    image: &RgbaImage,
    sigma: f32,
    radius: f32,
) -> RgbaImage {
    if sigma <= 0.0 || radius <= 0.0 {
        return image.clone();
    }

    let width = image.width() as usize;
    let height = image.height() as usize;

    // Single radius drives both axes, so clamp against the smaller dimension.
    let r = clamp_radius(radius as usize, width.min(height));
    if r == 0 {
        return image.clone();
    }
    let passes = derive_passes(sigma, r);

    box_blur_gaussian(image, r, passes, r, passes)
}

/// Create a new `RgbaImage` with a symmetric Gaussian blur effect.
/// This will give a smooth blur effect across the image.
///
/// Box radius is set to `sigma`, which yields ~3 passes — the central-limit sweet spot
/// where the box approximation is closest to a true Gaussian.
pub fn rgba_image_from_gaussian_blur(image: &RgbaImage, sigma: f32) -> RgbaImage {
    rgba_image_from_hoz_box_gaussian_blur(image, sigma, sigma)
}

/// Create a new `RgbaImage` with an asymmetric Gaussian blur.
/// `sigma` is the horizontal std dev, `radius` the vertical one, so unequal values give a
/// directional/stretched blur. A zero on either axis leaves that axis unblurred.
pub fn rgba_image_from_gaussian_asymmetric(
    image: &RgbaImage,
    sigma: f32,
    radius: f32,
) -> RgbaImage {
    if sigma <= 0.0 && radius <= 0.0 {
        return image.clone();
    }

    let width = image.width() as usize;
    let height = image.height() as usize;

    // Per-axis box radius = that axis's target sigma, clamped to its own dimension.
    let r_h = clamp_radius(sigma.max(0.0) as usize, width);
    let r_v = clamp_radius(radius.max(0.0) as usize, height);

    box_blur_gaussian(
        image,
        r_h,
        derive_passes(sigma, r_h),
        r_v,
        derive_passes(radius, r_v),
    )
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
pub fn rgba_image_from_pixelate(image: &RgbaImage, block_size: u32) -> RgbaImage {
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

/// Average each `scale`x`scale` block into one pixel — a cheap box downsample to `(w/scale, h/scale)`.
/// One linear pass over the source; trailing edge pixels that don't fill a block are dropped.
fn box_downscale(image: &RgbaImage, scale: usize) -> RgbaImage {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let sw = (w / scale).max(1);
    let sh = (h / scale).max(1);
    let src = image.as_raw();
    let mut out = vec![0u8; sw * sh * 4];

    for sy in 0..sh {
        let y0 = sy * scale;
        let y1 = (y0 + scale).min(h);
        for sx in 0..sw {
            let x0 = sx * scale;
            let x1 = (x0 + scale).min(w);
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for y in y0..y1 {
                let row = y * w * 4;
                for x in x0..x1 {
                    let i = row + x * 4;
                    r += src[i] as u32;
                    g += src[i + 1] as u32;
                    b += src[i + 2] as u32;
                    a += src[i + 3] as u32;
                }
            }
            let count = ((y1 - y0) * (x1 - x0)) as u32;
            let o = (sy * sw + sx) * 4;
            out[o] = (r / count) as u8;
            out[o + 1] = (g / count) as u8;
            out[o + 2] = (b / count) as u8;
            out[o + 3] = (a / count) as u8;
        }
    }
    RgbaImage::from_raw(sw as u32, sh as u32, out).unwrap()
}

/// Nearest-neighbour upscale of `small` to `width`x`height` — each source pixel becomes a block.
/// One linear pass over the output; edge sampling is clamped to the source bounds.
fn nearest_upscale(small: &RgbaImage, width: usize, height: usize, scale: usize) -> RgbaImage {
    let sw = small.width() as usize;
    let sh = small.height() as usize;
    let src = small.as_raw();
    let mut out = vec![0u8; width * height * 4];

    for y in 0..height {
        let sy = (y / scale).min(sh - 1);
        let srow = sy * sw * 4;
        let orow = y * width * 4;
        for x in 0..width {
            let si = srow + (x / scale).min(sw - 1) * 4;
            let oi = orow + x * 4;
            out[oi] = src[si];
            out[oi + 1] = src[si + 1];
            out[oi + 2] = src[si + 2];
            out[oi + 3] = src[si + 3];
        }
    }
    RgbaImage::from_raw(width as u32, height as u32, out).unwrap()
}

/// Fast, lower-fidelity blur for obscuring (not an exact Gaussian). Box-downsamples the image,
/// box-blurs at low resolution, then nearest-upscales back. Downsampling discards the high-frequency
/// detail that makes content recognizable; the low-res blur softens block-to-block transitions.
/// All three steps are single linear passes — far cheaper than a full-resolution blur. Intended for
/// the lock screen's "can't make it out" goal, where exactness does not matter.
pub fn rgba_image_from_box_gaussian_fast(image: &RgbaImage, sigma: f32, radius: f32) -> RgbaImage {
    if sigma <= 0.0 || radius <= 0.0 {
        return image.clone();
    }

    const SCALE: usize = 8;
    let width = image.width() as usize;
    let height = image.height() as usize;

    let small = box_downscale(image, SCALE);
    // Light box blur at low res; sigma/radius scale down with the image.
    let small = rgba_image_from_hoz_box_gaussian_blur(
        &small,
        (sigma / SCALE as f32).max(1.0),
        (radius / SCALE as f32).max(1.0),
    );
    nearest_upscale(&small, width, height, SCALE)
}

/// Apply one of the available `EffectType`(s) to the `RgbaImage`.
///
/// `sigma` is the standard deviation.
///
/// `radius` is distance of which the blur effect is spread across.
///
/// `fast` routes the blur effects through [`rgba_image_from_box_gaussian_fast`] — a cheaper,
/// approximate blur that trades fidelity for speed. `Pixelate` ignores it (already cheap).
pub(crate) fn apply_effect(
    image: &RgbaImage,
    sigma: f32,
    radius: f32,
    effect: EffectType,
    fast: bool,
) -> RgbaImage {
    match effect {
        EffectType::Gaussian if fast => rgba_image_from_box_gaussian_fast(image, sigma, sigma),
        EffectType::Gaussian => rgba_image_from_gaussian_blur(image, sigma),
        EffectType::GaussianAsymmetric if fast => {
            rgba_image_from_box_gaussian_fast(image, sigma, radius)
        }
        EffectType::GaussianAsymmetric => rgba_image_from_gaussian_asymmetric(image, sigma, radius),
        EffectType::GaussianBoxBlur if fast => {
            rgba_image_from_box_gaussian_fast(image, sigma, radius)
        }
        EffectType::GaussianBoxBlur => rgba_image_from_hoz_box_gaussian_blur(image, sigma, radius),
        EffectType::Pixelate => rgba_image_from_pixelate(image, sigma as u32),
    }
}
