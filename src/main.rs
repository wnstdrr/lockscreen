use clap::Parser;
use lockscreen::display;
use lockscreen::effect::EffectType;
use lockscreen::lock::{I3Lock, LockerBackend, ScreenLocker};
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

    #[clap(short, long, default_value = "i3")]
    backend: LockerBackend,

    #[clap(short, long, default_value = "false")]
    fast: bool,

    #[clap(long, default_value = "false")]
    no_lock: bool,
}

fn main() {
    let args = Args::parse();

    let displays = display::get_screenshots(args.effect, args.sigma, args.radius, args.fast);
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

    let locker: Box<dyn ScreenLocker> = match args.backend {
        LockerBackend::I3 => Box::new(I3Lock),
    };

    if let Err(e) = locker.lock(&path) {
        eprintln!("Failed to lock screen: {}", e.to_string());
    }
}
