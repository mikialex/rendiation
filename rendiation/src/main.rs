use crate::rinecraft::Rinecraft;

extern crate log;
extern crate winit;

mod application;
mod rinecraft;

fn main() {
    application::run::<Rinecraft>("rinecraft");
}
