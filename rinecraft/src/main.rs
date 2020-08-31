#![allow(dead_code)]
#![allow(unused)]

mod rinecraft;
// mod gui;
mod camera_controls;
mod effect;
mod init;
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

use rendiation_ral::BindGroupHandle;
use rendiation_shadergraph_derives::Shader;
use rendiation_webgpu::*;
use shading::BlockShadingParamGroup;
