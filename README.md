# Lockscreen

Create an effected lockscreen for the I3 desktop.

<img src="example/tmp_13360943157209490939.png" width="49%" alt="Desktop 1">
<img src="example/tmp_3898670366302154066.png" width="49%" alt="Desktop 2">

# Prerequisites

`i3` or `i3-gaps` with `i3lock` as default desktop locker.

For more information on getting started with i3 see their documentation [here](https://i3wm.org/docs/userguide.html).

Rust toolchain via [rustup](https://rustup.rs). If no default toolchain is set:

```shell
rustup default stable
```

`libclang` and `clang` for building bindgen-based dependencies (`libspa-sys`, `pipewire-sys`). On Void Linux:

```shell
sudo xbps-install -S libclang19 clang19
```

On Debian/Ubuntu:

```shell
sudo apt install libclang-dev
```

# Installation

Clone the repo and run the installation script. Requires `cargo` and `i3lock` on `PATH`.

```shell
git clone https://github.com/wnstdrr/lockscreen
cd lockscreen
sudo ./install.sh
```

This builds a release binary and installs it to `/usr/local/bin/lockscreen` along with the man page at `/usr/local/share/man/man1/lockscreen.1`.

To install to a custom location without `sudo`:

```shell
BIN_DIR=~/.local/bin MAN_DIR=~/.local/share/man/man1 ./install.sh
```

## i3 Config

Add a keybinding to your i3 config. My mod key is `Mod1` (left alt).

```shell
bindsym $mod+Ctrl+Shift+l exec lockscreen -s 1.5 -r 2.0 -e gaussian-asymmetric
```

## Dependencies

* [clap](https://crates.io/crates/clap/4.6.1) for command line argument parsing
* [xcap](https://crates.io/crates/xcap/0.9.4) for desktop screen capturing

## Dev Dependencies (Benchmark)

* [divan](https://crates.io/crates/divan/0.1.21) for benchmarking
* [fastblur](https://crates.io/crates/fastblur/0.1.1) for Gaussian blur benchmarks
* [libblur](https://crates.io/crates/libblur/0.24.0) for Gaussian blur benchmarks


# Blur Types

Currently, only supports three blur types.

* Gaussian

* Gaussian Asymmetric

* Pixelate

Gaussian and Gaussian Asymmetric effects provided by the [fastblur](https://crates.io/crates/fastblur) library.

## Blur benchmarks

1920×1080 RGBA, σ=10, 100 samples ([divan](https://docs.rs/divan), release build). Sorted by median.

| blur | fastest | slowest | median | mean |
|---|---|---|---|---|
| lockscreen_pixelate | 2.726 ms | 7.45 ms | 2.803 ms | 2.976 ms |
| lockscreen_gaussian_fast | 5.448 ms | 10.31 ms | 6.271 ms | 6.278 ms |
| lockscreen_gaussian_asymmetric | 8.238 ms | 12.52 ms | 9.1 ms | 9.284 ms |
| lockscreen_gaussian | 8.128 ms | 12.56 ms | 9.248 ms | 9.423 ms |
| lockscreen_gaussian_box_blur | 8.278 ms | 10.77 ms | 9.287 ms | 9.343 ms |
| libblur_gaussian_mt | 6.736 ms | 19.99 ms | 9.433 ms | 9.944 ms |
| libblur_gaussian_st | 23.09 ms | 27.51 ms | 24.88 ms | 24.97 ms |
| fastblur_gaussian | 47.63 ms | 50.3 ms | 48.9 ms | 48.9 ms |

Run with `cargo bench --bench blur`.


# Todo

* [X] Support i3lock
* [ ] Support GDM
* [ ] Migrate effects to library