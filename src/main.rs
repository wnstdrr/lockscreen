mod display;
mod effect;

use crate::effect::EffectType;
use clap::Parser;
use std::process::exit;

#[derive(Parser)]
#[command(version, about = "Screensaver utility for i3lock(1)", long_about = None)]
struct Args {
    #[clap(short, long, default_value = "1.5")]
    sigma: f32,

    #[clap(short, long, default_value = "0.1")]
    radius: f32,

    #[clap(short, long, default_value = "gaussian")]
    effect: EffectType,

    #[clap(long, default_value = "false")]
    no_lock: bool,
}

fn main() {
    let args = Args::parse();

    let displays = display::get_screenshots(args.effect, args.sigma, args.radius);
    let composite = display::compose_displays(&displays);
    let path = match display::save_composite(&composite) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to save composite display: {}", e.to_string());
            exit(1);
        }
    };

    if args.no_lock {
        println!("{}", path.display());
        exit(0);
    }

    if let Err(e) = display::lock_screen(&path) {
        eprintln!("Failed to open i3lock: {}", e);
        exit(1);
    }

    if let Err(e) = std::fs::remove_file(&path) {
        eprintln!("Failed to remove screen: {}", e);
        exit(1);
    }
}
