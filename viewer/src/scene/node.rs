use std::{cell::RefCell, rc::Rc};

use arena_tree::ArenaTree;
use rendiation_algebra::*;
use rendiation_controller::Transformed3DControllee;
use rendiation_webgpu::*;

use super::SceneNodeHandle;

pub struct SceneNodeData {
  pub visible: bool,
  pub local_matrix: Mat4<f32>,
  pub net_visible: bool,
  pub world_matrix: Mat4<f32>,
  pub gpu: Option<TransformGPU>,
}

impl Default for SceneNodeData {
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

impl Transformed3DControllee for SceneNodeData {
  fn matrix(&self) -> &Mat4<f32> {
    &self.local_matrix
  }

  fn matrix_mut(&mut self) -> &mut Mat4<f32> {
    &mut self.local_matrix
  }
}

impl SceneNodeData {
  pub fn hierarchy_update(&mut self, gpu: &GPU, parent: Option<&Self>) {
    if let Some(parent) = parent {
      self.net_visible = self.visible && parent.net_visible;
      if self.net_visible {
        self.world_matrix = parent.world_matrix * self.local_matrix;
      }
    } else {
      self.world_matrix = self.local_matrix;
      self.net_visible = self.visible
    }

    if self.net_visible {
      if let Some(t) = &mut self.gpu {
        t.update(gpu, &self.world_matrix);
      }
    }
  }

  pub fn get_model_gpu(&mut self, gpu: &GPU) -> &TransformGPU {
    self
      .gpu
      .get_or_insert_with(|| TransformGPU::new(gpu, &self.world_matrix))
  }

  pub fn set_position(&mut self, position: (f32, f32, f32)) -> &mut Self {
    self.local_matrix = Mat4::translate(position.0, position.1, position.2); // todo
    self
  }
}

#[derive(Clone)]
struct SceneNodeRef {
  nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>,
  handle: SceneNodeHandle,
}

impl Drop for SceneNodeRef {
  fn drop(&mut self) {
    let mut nodes = self.nodes.borrow_mut();
    nodes.free_node(self.handle)
  }
}

pub struct SceneNode {
  nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>,
  parent: Option<Rc<SceneNodeRef>>,
  inner: Rc<SceneNodeRef>,
}

impl SceneNode {
  pub fn from_root(nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>) -> Self {
    let nodes_info = nodes.borrow();
    let root = SceneNodeRef {
      nodes: nodes.clone(),
      handle: nodes_info.root(),
    };
    Self {
      nodes: nodes.clone(),
      parent: None,
      inner: Rc::new(root),
    }
  }

  pub fn create_child(&self) -> SceneNode {
    let mut nodes_info = self.nodes.borrow_mut();
    let handle = nodes_info.create_node(SceneNodeData::default());
    let inner = SceneNodeRef {
      nodes: self.nodes.clone(),
      handle,
    };

    nodes_info.node_add_child_by_id(self.inner.handle, handle);

    Self {
      nodes: self.nodes.clone(),
      parent: Some(self.inner.clone()),
      inner: Rc::new(inner),
    }
  }

  pub fn mutate<F: FnMut(&mut SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let mut nodes = self.nodes.borrow_mut();
    let node = nodes.get_node_mut(self.inner.handle).data_mut();
    f(node)
  }

  pub fn visit<F: FnMut(&SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let nodes = self.nodes.borrow();
    let node = nodes.get_node(self.inner.handle).data();
    f(node)
  }
}

impl Drop for SceneNode {
  fn drop(&mut self) {
    let mut nodes = self.nodes.borrow_mut();
    if let Some(parent) = self.parent.as_ref() {
      nodes.node_remove_child_by_id(parent.handle, self.inner.handle);
    }
  }
}

pub struct TransformGPU {
  pub cache: Mat4<f32>,
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

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
      [[block]]
      struct ModelTransform {{
        matrix: mat4x4<f32>;
      }};

      [[group({group}), binding(0)]]
      var<uniform> model: ModelTransform;
    
    ",
      group = group
    )
  }
}

impl TransformGPU {
  pub fn update(&mut self, gpu: &GPU, matrix: &Mat4<f32>) -> &mut Self {
    if self.cache == *matrix {
      return self;
    }
    self.cache = *matrix;

    gpu
      .queue
      .write_buffer(&self.ubo, 0, bytemuck::cast_slice(matrix.as_ref()));

    self
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

    Self {
      ubo,
      bindgroup,
      cache: *matrix,
    }
  }
}
