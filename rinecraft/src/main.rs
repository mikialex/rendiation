#![allow(dead_code)]

mod rinecraft;
// mod gui;
mod shading;
mod util;
mod watch;
mod vox;
mod sky;
mod init;
use rinecraft::*;
use rendium::application;

fn main() {
    env_logger::init();
    application::run::<Rinecraft>("rinecraft");
}
