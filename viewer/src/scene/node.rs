use rendiation_algebra::*;
use rendiation_controller::Transformed3DControllee;
use rendiation_webgpu::*;

use super::{Scene, SceneNodeHandle};

pub struct SceneNode {
  pub visible: bool,
  pub local_matrix: Mat4<f32>,
  pub net_visible: bool,
  pub world_matrix: Mat4<f32>,
  pub gpu: Option<TransformGPU>,
}

impl Default for SceneNode {
  fn default() -> Self {
    Self {
      visible: true,
      local_matrix: Mat4::one(),
      net_visible: true,
      world_matrix: Mat4::one(),
      gpu: None,
    }
  }
}

impl Transformed3DControllee for SceneNode {
  fn matrix(&self) -> &Mat4<f32> {
    &self.world_matrix
  }

  fn matrix_mut(&mut self) -> &mut Mat4<f32> {
    &mut self.world_matrix
  }
}

impl SceneNode {
  pub fn hierarchy_update(&mut self, parent: Option<&Self>) {
    if let Some(parent) = parent {
      self.net_visible = self.visible && parent.net_visible;
      if self.net_visible {
        self.world_matrix = parent.world_matrix * self.local_matrix;
      }
    } else {
      self.world_matrix = self.local_matrix;
      self.net_visible = self.visible
    }
  }

  pub fn get_model_gpu(&mut self, gpu: &GPU) -> (&Mat4<f32>, &TransformGPU) {
    (
      &self.world_matrix,
      self
        .gpu
        .get_or_insert_with(|| TransformGPU::new(gpu, &self.world_matrix)),
    )
  }

  pub fn set_position(&mut self, position: (f32, f32, f32)) -> &mut Self {
    self.local_matrix = Mat4::translate(position.0, position.1, position.2); // todo
    self
  }
}

impl Scene {
  pub fn get_root_handle(&self) -> SceneNodeHandle {
    self.nodes.get_node(self.nodes.root()).handle()
  }
  pub fn get_root(&self) -> &SceneNode {
    self.nodes.get_node(self.nodes.root()).data()
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode {
    self.get_node_mut(self.nodes.root())
  }

  pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle) {
    self.node_add_child_by_handle(self.nodes.root(), child_handle);
  }

  pub fn node_add_child_by_handle(
    &mut self,
    parent_handle: SceneNodeHandle,
    child_handle: SceneNodeHandle,
  ) {
    let (parent, child) = self
      .nodes
      .get_parent_child_pair(parent_handle, child_handle);
    parent.add(child);
  }

  pub fn node_remove_child_by_handle(
    &mut self,
    parent_handle: SceneNodeHandle,
    child_handle: SceneNodeHandle,
  ) {
    let (parent, child) = self
      .nodes
      .get_parent_child_pair(parent_handle, child_handle);
    parent.remove(child);
  }

  pub fn get_node(&self, handle: SceneNodeHandle) -> &SceneNode {
    self.nodes.get_node(handle).data()
  }

  pub fn get_node_mut(&mut self, handle: SceneNodeHandle) -> &mut SceneNode {
    self.nodes.get_node_mut(handle).data_mut()
  }

  pub fn create_new_node(&mut self) -> &mut SceneNode {
    let node = SceneNode::default();
    let handle = self.nodes.create_node(node);
    self.nodes.get_node_mut(handle).data_mut()
  }

  pub fn free_node(&mut self, handle: SceneNodeHandle) {
    self.nodes.free_node(handle);
  }
}

pub struct TransformGPU {
  pub ubo: wgpu::Buffer,
  pub bindgroup: wgpu::BindGroup,
  pub layout: wgpu::BindGroupLayout,
}

impl TransformGPU {
  pub fn get_shader_header() -> &'static str {
    r#"
      [[block]]
      struct ModelTransform {
          matrix: mat4x4<f32>;
      };
      [[group(0), binding(0)]]
      var model: ModelTransform;
    "#
  }

  pub fn update(&mut self, gpu: &GPU, matrix: &Mat4<f32>) {
    gpu
      .queue
      .write_buffer(&self.ubo, 0, bytemuck::cast_slice(matrix.as_ref()));
  }
  pub fn new(gpu: &GPU, matrix: &Mat4<f32>) -> Self {
    let device = &gpu.device;
    use wgpu::util::DeviceExt;

    let ubo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: "ModelTransformBindgroup Buffer".into(),
      contents: bytemuck::cast_slice(matrix.as_ref()),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: "ModelTransformBindgroup".into(),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: wgpu::BufferSize::new(64),
        },
        count: None,
      }],
    });

    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: ubo.as_entire_binding(),
      }],
      label: None,
    });

    Self {
      ubo,
      bindgroup,
      layout,
    }
  }
}
