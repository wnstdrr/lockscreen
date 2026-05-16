mod display;
mod effect;

use crate::effect::EffectType;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about = "Command line screensaver utility", long_about = None)]
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
    let path = display::save_composite(&composite);

    if args.no_lock {
        println!("{}", path.display());
    } else {
        display::lock_screen(&path);
        std::fs::remove_file(&path).unwrap();
    }
}
