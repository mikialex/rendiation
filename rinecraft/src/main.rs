#![allow(dead_code)]
#![allow(unused)]

mod application;
mod camera;
mod camera_controls;
mod init;
mod rendering;
mod rinecraft;
mod shading;
mod util;
mod vox;
mod window_event;
mod window_states;
use rinecraft::*;

#[tokio::main]
async fn main() {
  env_logger::init();
  application::run::<Rinecraft>("rinecraft");
}
