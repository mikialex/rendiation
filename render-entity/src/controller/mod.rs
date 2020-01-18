pub trait Controller<T>{
    fn update(&self, controlled: T);
}

pub mod orbit;
pub use orbit::*;