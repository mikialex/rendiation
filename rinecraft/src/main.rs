use rendiation::*;

mod test_renderer;
mod rinecraft;
mod geometry;
mod util;
mod vertex;
use test_renderer::*;
use rinecraft::*;

fn main() {
    env_logger::init();
    application::run::<TestRenderer, Rinecraft>("rinecraft");
}
