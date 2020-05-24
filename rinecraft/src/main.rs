#![allow(dead_code)]
#![allow(unused)]

mod rinecraft;
// mod gui;
mod shading;
mod util;
mod vox;
mod sky;
mod init;
mod camera_controls;
use rinecraft::*;
use rendium::application;

fn main() {
    env_logger::init();
    application::run::<Rinecraft>("rinecraft");
}
