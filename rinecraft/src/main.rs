#![allow(dead_code)]
#![allow(unused)]

mod rinecraft;
// mod gui;
mod shading;
mod util;
mod vox;
mod init;
mod camera_controls;
use rinecraft::*;
use rendium::application;

#[tokio::main]
async fn main() {
// fn main(){
    env_logger::init();
    application::run::<Rinecraft>("rinecraft");
}
