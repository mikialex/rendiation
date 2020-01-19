pub trait Controller<T>{
    fn update(&mut self, target: &mut T);
}

pub mod orbit;
pub use orbit::*;