use crate::*;

#[derive(Clone)]
pub enum ShaderInputNode {
  BuiltIn(ShaderBuiltInDecorator),
  UserDefinedIn {
    ty: PrimitiveShaderValueType,
    location: usize,
    // must have value for fragment in
    interpolation: Option<ShaderInterpolation>,
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
  pub fn insert_api_raw(self) -> ShaderNodeRawHandle {
    call_shader_api(|g| g.define_module_input(self))
  }
}

/// https://www.w3.org/TR/WGSL/#builtin-inputs-outputs
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShaderBuiltInDecorator {
  VertexIndex,
  VertexInstanceIndex,
  VertexPositionOut,
  FragPositionIn,
  FragFrontFacing,
  FragDepth,
  FragSampleIndex,
  FragSampleMask,
  CompSubgroupInvocationId,
  CompLocalInvocationId,
  CompGlobalInvocationId,
  CompLocalInvocationIndex,
  CompSubgroupId,
  CompWorkgroupId,
  CompNumWorkgroup,
  CompSubgroupSize,
}

#[derive(Default, Clone)]
pub struct ShaderBindGroup {
  pub bindings: Vec<ShaderBindEntry>,
}

#[derive(Clone)]
pub struct ShaderBindEntry {
  pub desc: ShaderBindingDescriptor,
  pub visibility: ShaderStages,
  pub bindgroup_index: usize,
  pub entry_index: usize,
  pub vertex_node: Option<ShaderNodeRawHandle>,
  pub fragment_node: Option<ShaderNodeRawHandle>,
  pub compute_node: Option<ShaderNodeRawHandle>,
}

impl ShaderBindEntry {
  pub fn using(&mut self) -> ShaderNodeRawHandle {
    let current_stage = get_current_stage().expect("must in shader building");

    let node = match current_stage {
      ShaderStage::Vertex => &mut self.vertex_node,
      ShaderStage::Fragment => &mut self.fragment_node,
      ShaderStage::Compute => &mut self.compute_node,
    };

    *node.get_or_insert_with(|| {
      let bit = match current_stage {
        ShaderStage::Vertex => ShaderStages::VERTEX,
        ShaderStage::Fragment => ShaderStages::FRAGMENT,
        ShaderStage::Compute => ShaderStages::COMPUTE,
      };

      self.visibility.insert(bit);

      let input = ShaderInputNode::Binding {
        desc: self.desc.clone(),
        bindgroup_index: self.bindgroup_index,
        entry_index: self.entry_index,
      };

      input.insert_api_raw()
    })
  }
}

/// provide a single binding source in shader, should impl by base container ty
pub trait ShaderBindingProvider {
  type Node: ShaderNodeType;
  type ShaderInstance: Clone = Node<Self::Node>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance;
  fn binding_desc(&self) -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty: Self::Node::ty(),
    }
  }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ShaderBindingDescriptor {
  pub should_as_storage_buffer_if_is_buffer_like: bool,
  pub ty: ShaderValueType,
  pub writeable_if_storage: bool,
}

impl ShaderBindingDescriptor {
  pub fn get_buffer_layout(&self) -> Option<StructLayoutTarget> {
    match &self.ty {
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
        ty: ShaderValueType::Single(ty.clone()),
      }
      .get_buffer_layout(),
      ShaderValueType::Never => None,
    }
  }

  pub fn get_address_space(&self) -> Option<AddressSpace> {
    match &self.ty {
      ShaderValueType::Single(ty) => match ty {
        ShaderValueSingleType::Sized(_) => {
          if self.should_as_storage_buffer_if_is_buffer_like {
            AddressSpace::Storage {
              writeable: self.writeable_if_storage,
            }
          } else {
            AddressSpace::Uniform
          }
        }
        ShaderValueSingleType::Unsized(_) => AddressSpace::Storage {
          writeable: self.writeable_if_storage,
        },
        _ => AddressSpace::Handle,
      },
      ShaderValueType::BindingArray { .. } => AddressSpace::Handle,
      ShaderValueType::Never => return None,
    }
    .into()
  }
}

impl<T: ShaderBindingProvider> ShaderBindingProvider for &T {
  type Node = T::Node;
  type ShaderInstance = T::ShaderInstance;

  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    (*self).create_instance(node)
  }

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
  type ShaderInstance = T::ShaderInstance;

  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    self.0.create_instance(node)
  }

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
