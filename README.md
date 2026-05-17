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
* [fastblur](https://crates.io/crates/fastblur/0.1.1) for fast Gaussian blur effects
* [xcap](https://crates.io/crates/xcap/0.9.4) for desktop screen capturing

# Blur Types

Currently, only supports three blur types.

* Gaussian

* Gaussian Asymmetric

* Pixelate

Gaussian and Gaussian Asymmetric effects provided by the [fastblur](https://crates.io/crates/fastblur) library.