use rendiation::*;

mod rinecraft;
mod shading;
mod util;
mod watch;
mod vox;
use rinecraft::*;

fn main() {
    env_logger::init();
    application::run::<Rinecraft>("rinecraft");
}
