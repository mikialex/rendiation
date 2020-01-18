pub trait Controller<T>{
    fn update(controlled: T);
}

pub mod orbit;
pub use orbit::*;