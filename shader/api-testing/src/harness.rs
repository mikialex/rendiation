use std::sync::Arc;

use naga::valid::{Capabilities, ValidationFlags, Validator};
use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_shader_backend_naga::*;

/// Build a compute shader module by the naga backend, the logic is written in the entry function.
pub fn build_compute(logic: impl FnOnce(&ShaderComputePipelineBuilder)) -> naga::Module {
  let builder = ShaderComputePipelineBuilder::new(
    &|stage| Box::new(ShaderAPINagaImpl::new(stage)),
    ShaderRuntimeChecks::default(),
  );
  logic(&builder);
  let result = builder.build().expect("failed to build shader");
  result
    .shader
    .1
    .downcast::<NagaModuleBuildResult>()
    .expect("expect naga backend build result")
    .module
}

/// Validate the module by the naga validator with all the validation flags.
pub fn validate(module: &naga::Module) {
  Validator::new(ValidationFlags::all(), Capabilities::all())
    .validate(module)
    .unwrap_or_else(|e| panic!("naga validation failed: {:?}", e.into_inner()));
}

/// Build a compute shader and validate it.
pub fn check_compute(logic: impl FnOnce(&ShaderComputePipelineBuilder)) {
  validate(&build_compute(logic))
}

/// Build a graphics shader by the naga backend, return the vertex and fragment shader module.
pub fn build_graphics(logic: impl Fn(&mut ShaderRenderPipelineBuilder)) -> [naga::Module; 2] {
  struct Logic<F>(F);
  impl<F: Fn(&mut ShaderRenderPipelineBuilder)> GraphicsShaderProvider for Logic<F> {
    fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
      (self.0)(builder)
    }
  }

  let builder = Logic(logic)
    .build_self(
      &|stage| Box::new(ShaderAPINagaImpl::new(stage)),
      None,
      Arc::new(fake_gpu_info()),
      ShaderRuntimeChecks::default(),
    )
    .unwrap_or_else(|e| panic!("failed to build shader: {e:?}"));
  let result = builder.build().expect("failed to build shader");

  let VertexOrTaskMesh::Vertex(vertex) = result.shape_shader else {
    unreachable!("expect vertex shader")
  };
  [vertex, result.frag_shader].map(|(_, shader)| {
    shader
      .downcast::<NagaModuleBuildResult>()
      .expect("expect naga backend build result")
      .module
  })
}

/// Build a graphics shader and validate the vertex and fragment shader.
pub fn check_graphics(logic: impl Fn(&mut ShaderRenderPipelineBuilder)) {
  build_graphics(logic).iter().for_each(validate);
}

fn fake_gpu_info() -> GPUInfo {
  GPUInfo {
    adaptor_info: wgpu_types::AdapterInfo {
      name: String::new(),
      vendor: 0,
      device: 0,
      device_type: wgpu_types::DeviceType::Other,
      device_pci_bus_id: String::new(),
      driver: String::new(),
      driver_info: String::new(),
      backend: wgpu_types::Backend::Noop,
      subgroup_min_size: 4,
      subgroup_max_size: 128,
      transient_saves_memory: false,
    },
    power_preference: Default::default(),
    supported_features: wgpu_types::Features::all(),
    supported_limits: Default::default(),
    downgrade_info: Default::default(),
  }
}

/// Store the value into a local variable, so the expression is used by a statement.
pub fn keep<T: ShaderSizedValueNodeType>(v: Node<T>) {
  v.make_local_var();
}

/// Create a binding in the bind group 0 without any GPU resource container.
pub fn fake_binding<T: ShaderNodeType>(entry_index: usize) -> Node<T> {
  fake_binding_by_ty(entry_index, T::ty())
}

/// Create a read_write storage buffer binding in the bind group 0 without any GPU resource
/// container.
pub fn fake_storage_buffer<T>(entry_index: usize) -> ShaderPtrOf<T>
where
  T: ShaderNodeType + ShaderAbstractPtrAccess + ?Sized,
{
  let handle = ShaderInputNode::Binding {
    desc: ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: T::ty(),
      writeable_if_storage: true,
      has_dynamic_offset: false,
    },
    bindgroup_index: 0,
    entry_index,
  }
  .insert_api_raw();
  T::create_view_from_raw_ptr(Box::new(handle))
}

/// The pointer of `T` at the u32 offset of the u32 heap (in std430 layout), the same pointer
/// implementation of the combined buffer.
pub fn u32_heap_ptr<T>(heap: ShaderPtrOf<[u32]>, offset: u32) -> ShaderPtrOf<T>
where
  T: ShaderNodeType + ShaderAbstractPtrAccess,
{
  let ShaderValueType::Single(ty) = T::ty() else {
    unreachable!("expect single type")
  };
  let ptr = U32HeapPtrWithType {
    ptr: U32HeapPtr {
      array: U32HeapHeapSource::Common(heap),
      offset: val(offset),
    },
    ty,
    array_length: None,
    meta: Arc::new(RwLock::new(ShaderU32StructMetaData::new(
      StructLayoutTarget::Std430,
    ))),
  };
  T::create_view_from_raw_ptr(Box::new(ptr))
}

/// Create a storage texture binding like [fake_binding], the storage format is not expressed in
/// the shader type, so it must be given here to match the channel type.
pub fn fake_storage_texture_binding<T: ShaderNodeType>(
  entry_index: usize,
  storage_format: StorageFormat,
) -> Node<T> {
  let mut ty = T::ty();
  ty.mutate_single(|ty| {
    if let ShaderValueSingleType::StorageTexture { format, .. } = ty {
      *format = storage_format;
    }
  });
  fake_binding_by_ty(entry_index, ty)
}

fn fake_binding_by_ty<T: ShaderNodeType>(entry_index: usize, ty: ShaderValueType) -> Node<T> {
  ShaderInputNode::Binding {
    desc: ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      ty,
      writeable_if_storage: false,
      has_dynamic_offset: false,
    },
    bindgroup_index: 0,
    entry_index,
  }
  .insert_api()
}

/// The values that are only known at runtime, so the tested expressions are not constant.
pub struct RuntimeValues {
  pub u: Node<u32>,
  pub i: Node<i32>,
  pub f: Node<f32>,
}

pub fn runtime_values(builder: &ShaderComputePipelineBuilder) -> RuntimeValues {
  let u = builder.global_invocation_id().x();
  RuntimeValues {
    u,
    i: u.into_i32(),
    f: u.into_f32(),
  }
}
