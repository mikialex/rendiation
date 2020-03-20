mod rinecraft;
// mod gui;
mod shading;
mod util;
mod watch;
mod vox;
mod sky;
mod init;
mod noise;
use rinecraft::*;
use rendium::application;

fn main() {
    env_logger::init();
    application::run::<Rinecraft>("rinecraft");
}
