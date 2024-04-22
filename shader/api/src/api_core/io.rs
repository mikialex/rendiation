use crate::*;

#[derive(Clone)]
pub enum ShaderInputNode {
  BuiltIn(ShaderBuiltInDecorator),
  UserDefinedIn {
    ty: PrimitiveShaderValueType,
    location: usize,
  },
  Binding {
    desc: ShaderBindingDescriptor,
    bindgroup_index: usize,
    entry_index: usize,
  },
  WorkGroupShared {
    ty: ShaderSizedValueType,
  },
  Private {
    ty: ShaderSizedValueType,
  },
}

impl ShaderInputNode {
  pub fn insert_api<T: ShaderNodeType + ?Sized>(self) -> Node<T> {
    call_shader_api(|g| unsafe { g.define_module_input(self).into_node() })
  }
}

/// https://www.w3.org/TR/WGSL/#builtin-inputs-outputs
#[derive(Debug, Copy, Clone)]
pub enum ShaderBuiltInDecorator {
  VertexIndex,
  VertexInstanceIndex,
  VertexPositionOut,
  FragPositionIn,
  FragFrontFacing,
  FragDepth,
  FragSampleIndex,
  FragSampleMask,
  CompLocalInvocationId,
  CompGlobalInvocationId,
  CompLocalInvocationIndex,
  CompWorkgroupId,
  CompNumWorkgroup,
}

#[derive(Default, Clone)]
pub struct ShaderBindGroup {
  pub bindings: Vec<ShaderBindEntry>,
}

#[derive(Clone, Copy)]
pub struct ShaderBindEntry {
  pub desc: ShaderBindingDescriptor,
  pub vertex_node: ShaderNodeRawHandle,
  pub fragment_node: ShaderNodeRawHandle,
  pub compute_node: ShaderNodeRawHandle,
}

/// should impl by user's container ty
pub trait ShaderBindingProvider {
  type Node: ShaderNodeType + ?Sized;
  fn binding_desc(&self) -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty: Self::Node::TYPE,
    }
  }
}

#[derive(Clone, Copy)]
pub struct ShaderBindingDescriptor {
  pub should_as_storage_buffer_if_is_buffer_like: bool,
  pub ty: ShaderValueType,
  pub writeable_if_storage: bool,
}

impl ShaderBindingDescriptor {
  pub fn get_buffer_layout(&self) -> Option<StructLayoutTarget> {
    match self.ty {
      ShaderValueType::Single(ty) => match ty {
        ShaderValueSingleType::Sized(_) => if self.should_as_storage_buffer_if_is_buffer_like {
          StructLayoutTarget::Std430
        } else {
          StructLayoutTarget::Std140
        }
        .into(),
        ShaderValueSingleType::Unsized(_) => StructLayoutTarget::Std430.into(),
        _ => None,
      },
      ShaderValueType::BindingArray { ty, .. } => ShaderBindingDescriptor {
        should_as_storage_buffer_if_is_buffer_like: self.should_as_storage_buffer_if_is_buffer_like,
        writeable_if_storage: false,
        ty: ShaderValueType::Single(ty),
      }
      .get_buffer_layout(),
      ShaderValueType::Never => None,
    }
  }
}

impl<'a, T: ShaderBindingProvider> ShaderBindingProvider for &'a T {
  type Node = T::Node;

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    (*self).binding_desc()
  }
}

/// https://www.w3.org/TR/webgpu/#texture-format-caps
/// not all format could be filtered, use this to override
/// todo, check runtime support and dynamically decide downgrade behavior
pub struct DisableFiltering<T>(pub T);

impl<T: ShaderBindingProvider> ShaderBindingProvider for DisableFiltering<T> {
  type Node = T::Node;
  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut ty = self.0.binding_desc();
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
