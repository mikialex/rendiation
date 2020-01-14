use rendiation::*;

mod rinecraft;
mod geometry;
mod shading;
mod image_data;
mod util;
mod vertex;
mod watch;
use rinecraft::*;

fn main() {
    env_logger::init();
    application::run::<Rinecraft>("rinecraft");
}
