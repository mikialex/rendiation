#![allow(dead_code)]
#![allow(unused)]

mod application;
mod camera;
mod camera_controls;
mod init;
mod rendering;
mod shading;
mod util;
mod vox;
mod voxland;
mod window_event;
mod window_states;
use voxland::*;

fn main() {
  env_logger::init();
  application::run::<Voxland>("voxland");
}
