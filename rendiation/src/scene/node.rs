pub struct SceneNode {
  transform_dirty_id: usize,
  self_id: Index,
  parent: Index,
  children: Vec<Index>
}

impl SceneNode {
  fn traverse(scene: Scene){

  }
}

pub trait RenderEntity{

}

pub struct Scene {
  background: Box<dyn Background>,
  active_camera_index: Index,
  cameras: Arena<Box<dyn Camera>>,

  render_objects: Arena<RenderObject>,
  nodes: Arena<SceneNode>,

  renderables_dynamic: Arena<Box<dyn Renderable>>,
  canvas: WGPUTexture,
}

pub trait TransformLocalWorld{
  fn get_local_transform();
  fn set_local_transform();
  fn get_world_transform();
  fn set_world_transform();
}

pub struct RenderObject{
  shading_index: Index,
  geometry_index: Index,

  world_bounding: Bounding,
  world_matrix: Mat4<f32>,
  local_matrix: Mat4<f32>,
  normal_matrix: Mat4<f32>,

}

pub struct ResourceManager{
  // shadings
  // geometries

}