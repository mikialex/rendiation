#![allow(dead_code)]
#![allow(unused)]

mod rinecraft;
// mod gui;
mod camera_controls;
mod init;
mod rendering;
mod shading;
mod util;
mod vox;
use rendium::application;
use rinecraft::*;

#[tokio::main]
async fn main() {
  env_logger::init();
  application::run::<Rinecraft>("rinecraft");
}
