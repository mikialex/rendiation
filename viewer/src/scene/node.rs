use std::{cell::RefCell, rc::Rc};

use arena_tree::ArenaTree;
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
    &self.local_matrix
  }

  fn matrix_mut(&mut self) -> &mut Mat4<f32> {
    &mut self.local_matrix
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

#[derive(Clone)]
pub struct SceneNodeRef {
  nodes: Rc<RefCell<ArenaTree<SceneNode>>>,
  handle: SceneNodeHandle,
  // parent: Option<Box<SceneNodeRef>>,
  // children: Vec<SceneNodeRef>,
}

impl Drop for SceneNodeRef {
  fn drop(&mut self) {
    let mut nodes = self.nodes.borrow_mut();
    // todo remove node
  }
}

impl Scene {
  pub fn create_node(&mut self, builder: impl Fn(&mut SceneNode, &mut Self)) -> SceneNodeHandle {
    let mut node = SceneNode::default();
    builder(&mut node, self);
    let mut nodes = self.components.nodes.borrow_mut();
    let new = nodes.create_node(node);
    let root = nodes.root();
    nodes.node_add_child_by_id(root, new);
    new
  }

  pub fn create_node2(&mut self) -> SceneNodeRef {
    let handle = self
      .components
      .nodes
      .borrow_mut()
      .create_node(SceneNode::default());
    SceneNodeRef {
      nodes: self.components.nodes.clone(),
      handle,
    }
  }

  pub fn get_root_handle(&self) -> SceneNodeHandle {
    self.components.nodes.borrow().get_root_node().handle()
  }
  // pub fn get_root(&self) -> &SceneNode {
  //   self.components.nodes.get_root_node().data()
  // }

  // pub fn get_root_node_mut(&mut self) -> &mut SceneNode {
  //   self.components.nodes.get_root_node_mut().data_mut()
  // }

  // pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle) {
  //   self.node_add_child_by_handle(self.components.nodes.root(), child_handle);
  // }

  // pub fn node_add_child_by_handle(
  //   &mut self,
  //   parent_handle: SceneNodeHandle,
  //   child_handle: SceneNodeHandle,
  // ) {
  //   let (parent, child) = self
  //     .components
  //     .nodes
  //     .get_parent_child_pair(parent_handle, child_handle);
  //   parent.add(child);
  // }

  // pub fn node_remove_child_by_handle(
  //   &mut self,
  //   parent_handle: SceneNodeHandle,
  //   child_handle: SceneNodeHandle,
  // ) {
  //   let (parent, child) = self
  //     .components
  //     .nodes
  //     .get_parent_child_pair(parent_handle, child_handle);
  //   parent.remove(child);
  // }

  // pub fn get_node(&self, handle: SceneNodeHandle) -> &SceneNode {
  //   self.components.nodes.get_node(handle).data()
  // }

  // pub fn get_node_mut(&mut self, handle: SceneNodeHandle) -> &mut SceneNode {
  //   self.components.nodes.get_node_mut(handle).data_mut()
  // }

  // pub fn create_new_node(&mut self) -> &mut SceneNode {
  //   let node = SceneNode::default();
  //   let handle = self.components.nodes.create_node(node);
  //   self.components.nodes.get_node_mut(handle).data_mut()
  // }

  // pub fn free_node(&mut self, handle: SceneNodeHandle) {
  //   self.components.nodes.free_node(handle);
  // }
}

pub struct TransformGPU {
  pub ubo: wgpu::Buffer,
  pub bindgroup: Rc<wgpu::BindGroup>,
}

impl BindGroupLayoutProvider for TransformGPU {
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
  }
}

impl TransformGPU {
  pub fn get_shader_header() -> &'static str {
    r#"
      [[block]]
      struct ModelTransform {
          matrix: mat4x4<f32>;
      };
      [[group(0), binding(0)]]
      var<uniform> model: ModelTransform;
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

    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &Self::layout(device),
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: ubo.as_entire_binding(),
      }],
      label: None,
    });

    let bindgroup = Rc::new(bindgroup);

    Self { ubo, bindgroup }
  }
}
