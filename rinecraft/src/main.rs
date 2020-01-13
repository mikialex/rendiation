use rendiation::*;

mod test_renderer;
mod rinecraft;
mod geometry;
mod texture;
mod shading;
mod util;
mod vertex;
mod watch;
use test_renderer::*;
use rinecraft::*;

fn main() {
    env_logger::init();
    application::run::<TestRenderer, Rinecraft>("rinecraft");
}
