use clap::ValueEnum;
use std::path::PathBuf;
use std::process::Command;

#[derive(ValueEnum, Clone)]
pub enum LockerBackend {
    I3,
    // TODO: Support Gnome lock screen / gdm
}

pub trait ScreenLocker {
    fn lock(&self, path: &PathBuf) -> Result<(), std::io::Error>;
}

pub struct I3Lock;
impl ScreenLocker for I3Lock {
    fn lock(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        Command::new("i3lock").arg("-i").arg(path).output()?;
        Ok(())
    }
}
