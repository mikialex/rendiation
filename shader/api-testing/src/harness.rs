use naga::valid::{Capabilities, ValidationFlags, Validator};
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

/// Store the value into a local variable, so the expression is used by a statement.
pub fn keep<T: ShaderSizedValueNodeType>(v: Node<T>) {
  v.make_local_var();
}

/// Create a binding in the bind group 0 without any GPU resource container.
pub fn fake_binding<T: ShaderNodeType>(entry_index: usize) -> Node<T> {
  fake_binding_by_ty(entry_index, T::ty())
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
