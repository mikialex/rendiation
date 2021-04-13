use sceno::SceneBackend;

pub struct WebGPUScene;

impl SceneBackend for WebGPUScene {
  type Model = Box<dyn Model>;
  type Material = Box<dyn Material>;
  type Mesh = Box<dyn Mesh>;
  type Background = Box<dyn Background>;
  type Light = Box<dyn Light>;
}

pub trait Light {}
pub trait Background {}
pub trait Mesh {}
pub trait Material {}
pub trait Model {}

pub trait GPUSceneExt {
  //
}

pub struct WebGPURenderer {
  pipeline_cache: Vec<wgpu::RenderPipeline>,
}
