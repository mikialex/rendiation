pub mod background;
pub mod culling;
pub mod node;
pub mod render_engine;
pub mod render_list;
pub mod render_object;
pub mod resource;
pub mod scene;

pub use background::*;
pub use culling::*;
pub use node::*;
pub use render_engine::*;
pub use render_list::*;
pub use render_object::*;
pub use resource::*;
pub use scene::*;

pub trait SceneGraphBackend {
  // resource type injection
  type RenderTarget;
  type Renderer;
  type Shading;
  type ShadingParameterGroup;
  type IndexBuffer;
  type VertexBuffer;
  type UniformBuffer;
  type UniformValue;
}
