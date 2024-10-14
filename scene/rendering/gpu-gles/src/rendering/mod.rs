//! extension points overview
//! scene model: adjust binding and top level dependency or render any strange stuff
//! model: render model like with concept like shape and material
//! specific topics: shape, material, camera, node

mod scene;
pub use scene::*;

mod scene_model;
pub use scene_model::*;

mod model;
pub use model::*;

mod node;
pub use node::*;

mod camera;
pub use camera::*;

mod material;
pub use material::*;

mod shape;
pub use shape::*;
