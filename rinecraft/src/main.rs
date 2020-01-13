use rendiation::*;

mod rinecraft;
mod geometry;
mod texture;
mod shading;
mod util;
mod vertex;
mod watch;
use rinecraft::*;

fn main() {
    env_logger::init();
    application::run::<Rinecraft>("rinecraft");
}
