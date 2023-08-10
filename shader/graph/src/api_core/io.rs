use crate::*;

#[derive(Clone)]
pub enum ShaderGraphInputNode {
  BuiltIn(ShaderBuiltIn),
  Binding {
    ty: ShaderValueType,
    bindgroup_index: usize,
    entry_index: usize,
  },
  VertexIn {
    ty: PrimitiveShaderValueType,
    location: usize,
  },
  FragmentIn {
    ty: PrimitiveShaderValueType,
    location: usize,
  },
}

impl ShaderGraphInputNode {
  pub fn insert_graph<T: ShaderGraphNodeType>(self) -> Node<T> {
    modify_graph(|g| unsafe { g.define_module_input(self).into_node() })
  }
}

#[derive(Copy, Clone)]
pub enum ShaderBuiltIn {
  VertexIndexId,
  VertexInstanceId,
  FragmentFrontFacing,
  FragmentSampleIndex,
  FragmentSampleMask,
  FragmentNDC,
}

#[derive(Default, Clone)]
pub struct ShaderGraphBindGroup {
  pub bindings: Vec<ShaderGraphBindEntry>,
}

#[derive(Clone, Copy)]
pub struct ShaderGraphBindEntry {
  pub desc: ShaderBindingDescriptor,
  pub vertex_node: ShaderGraphNodeRawHandle,
  pub fragment_node: ShaderGraphNodeRawHandle,
}

/// should impl by user's container ty
pub trait ShaderBindingProvider {
  type Node: ShaderGraphNodeType;
  fn binding_desc() -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      ty: Self::Node::TYPE,
    }
  }
}

#[derive(Clone, Copy)]
pub struct ShaderBindingDescriptor {
  pub should_as_storage_buffer_if_is_buffer_like: bool,
  pub ty: ShaderValueType,
}

impl<'a, T: ShaderBindingProvider> ShaderBindingProvider for &'a T {
  type Node = T::Node;

  fn binding_desc() -> ShaderBindingDescriptor {
    T::binding_desc()
  }
}

/// https://www.w3.org/TR/webgpu/#texture-format-caps
/// not all format could be filtered, use this to override
pub struct DisableFiltering<T>(pub T);

impl<T: ShaderBindingProvider> ShaderBindingProvider for DisableFiltering<T> {
  type Node = T::Node;
  fn binding_desc() -> ShaderBindingDescriptor {
    let mut ty = T::binding_desc();
    ty.ty.mutate_single(|ty| {
      if let ShaderValueSingleType::Texture {
        sample_type: TextureSampleType::Float { filterable },
        ..
      } = ty
      {
        *filterable = false;
      }

      if let ShaderValueSingleType::Sampler(ty) = ty {
        *ty = SamplerBindingType::NonFiltering
      }
    });

    ty
  }
}
