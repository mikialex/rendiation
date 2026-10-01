use crate::*;

/// The configuration and the outputs of the entry point, the naga entry point is assembled from
/// it when building, see [ShaderAPINagaImpl::finish_entry_point].
pub(crate) struct EntryState {
  stage: naga::ShaderStage,
  /// zero means not configured, the compute, task and mesh stages must configure it
  pub(crate) workgroup_size: [u32; 3],
  pub(crate) early_depth_test: Option<naga::EarlyDepthTest>,
  pub(crate) mesh_info: Option<naga::MeshStageInfo>,
  pub(crate) task_payload: Option<naga::Handle<naga::GlobalVariable>>,
  outputs: Vec<EntryOutput>,
  /// the location of the next user defined output, the builtin outputs do not take the location
  next_output_location: usize,
  /// the task stage returns the mesh task size instead of the outputs
  pub(crate) mesh_task_size: Option<ShaderNodeRawHandle>,
}

impl EntryState {
  pub(crate) fn new(stage: naga::ShaderStage) -> Self {
    Self {
      stage,
      workgroup_size: [0; 3],
      early_depth_test: None,
      mesh_info: None,
      task_payload: None,
      outputs: Default::default(),
      next_output_location: 0,
      mesh_task_size: None,
    }
  }
}

struct EntryOutput {
  meta: ShaderStructFieldMetaInfo,
  /// the local variable that holds the output value, it is loaded and composed into the output
  /// struct when the entry function returns
  var: naga::Handle<naga::Expression>,
}

pub(crate) const ENTRY_POINT_NAME: &str = "main";

impl ShaderAPINagaImpl {
  pub(crate) fn define_module_input_impl(&mut self, input: ShaderInputNode) -> ShaderNodeRawHandle {
    // the other inputs are global variables, they can be declared while building any function
    if matches!(
      input,
      ShaderInputNode::BuiltIn(_) | ShaderInputNode::UserDefinedIn { .. }
    ) {
      assert!(
        self.functions.len() == 1,
        "the built-in input and the user defined input are the entry function arguments, \
         they can not be defined while building a user function"
      );
    }
    match input {
      ShaderInputNode::BuiltIn(ty) => {
        let data_ty = ty
          .data_ty()
          .expect("mesh output relative should defined by shared var");
        let data_ty = self.register_primitive_ty(data_ty);
        self.add_fn_input_inner(naga::FunctionArgument {
          name: None,
          ty: data_ty,
          binding: naga::Binding::BuiltIn(map_built_in(ty)).into(),
        })
      }
      ShaderInputNode::Binding {
        desc,
        bindgroup_index,
        entry_index,
      } => {
        let space = map_address_space(desc.get_address_space().unwrap());
        if let ShaderValueType::BindingArray { ty, .. } = &desc.ty
          && space != naga::AddressSpace::Handle
        {
          assert!(
            matches!(
              ty,
              ShaderValueSingleType::Sized(ShaderSizedValueType::Struct(_))
                | ShaderValueSingleType::Unsized(ShaderUnSizedValueType::UnsizedStruct(_))
            ),
            "the element of the buffer binding array must be a struct, got: {ty:?}"
          );
        }
        let ty = self.register_ty_impl(desc.ty);
        let binding = naga::ResourceBinding {
          group: bindgroup_index as u32,
          binding: entry_index as u32,
        };
        self.declare_global(space, Some(binding), ty)
      }
      ShaderInputNode::UserDefinedIn {
        ty,
        location,
        interpolation,
      } => {
        let ty = self.register_primitive_ty(ty);
        self.add_fn_input_inner(naga::FunctionArgument {
          name: None,
          ty,
          binding: naga::Binding::Location {
            location: location as u32,
            interpolation: interpolation.map(map_interpolation),
            sampling: None,
            blend_src: None,
            per_primitive: false,
          }
          .into(),
        })
      }
      ShaderInputNode::WorkGroupShared { ty } => {
        let ty = self.register_sized_ty(ty);
        self.declare_global(naga::AddressSpace::WorkGroup, None, ty)
      }
      ShaderInputNode::Private { ty } => {
        let ty = self.register_sized_ty(ty);
        self.declare_global(naga::AddressSpace::Private, None, ty)
      }
      ShaderInputNode::TaskPayload { ty } => {
        let ty = self.register_sized_ty(ty);
        self.declare_global(naga::AddressSpace::TaskPayload, None, ty)
      }
    }
  }

  pub(crate) fn define_out(
    &mut self,
    ty: ShaderSizedValueType,
    name: String,
    ty_deco: ShaderFieldDecorator,
  ) -> ShaderNodeRawHandle {
    assert!(
      self.functions.len() == 1 && self.building_fn().frames.len() == 1,
      "the shader output must be defined in the root scope of the entry function"
    );

    let node = self.make_local_var(ShaderValueType::Single(ShaderValueSingleType::Sized(
      ty.clone(),
    )));
    let var = self.get_expression(node);
    self.entry.outputs.push(EntryOutput {
      meta: ShaderStructFieldMetaInfo {
        name,
        ty,
        ty_deco: Some(ty_deco),
      },
      var,
    });
    node
  }

  pub(crate) fn define_location_out(
    &mut self,
    ty: ShaderSizedValueType,
    name_prefix: &str,
    interpolation: Option<ShaderInterpolation>,
  ) -> ShaderNodeRawHandle {
    let location = self.entry.next_output_location;
    self.entry.next_output_location += 1;
    self.define_out(
      ty,
      format!("{name_prefix}_{location}"),
      ShaderFieldDecorator::Location(location, interpolation),
    )
  }

  /// Close the entry function and assemble the entry point into the module, only the entry
  /// function body can be left open.
  pub(crate) fn finish_entry_point(&mut self) {
    assert!(
      self.functions.len() == 1 && self.building_fn().frames.len() == 1,
      "the shader scopes are not balanced when building, some scope or function is not closed"
    );
    let stage = self.entry.stage;
    if matches!(
      stage,
      naga::ShaderStage::Compute | naga::ShaderStage::Task | naga::ShaderStage::Mesh
    ) {
      assert!(
        self.entry.workgroup_size.iter().all(|v| *v > 0),
        "the workgroup size of the {stage:?} stage is not configured"
      );
    }

    let result = self.return_entry_outputs();
    let mut function = self.functions.pop().unwrap().finish();
    function.result = result;

    let entry = &mut self.entry;
    self.module.entry_points.push(naga::EntryPoint {
      name: ENTRY_POINT_NAME.to_owned(),
      stage,
      early_depth_test: entry.early_depth_test,
      workgroup_size: entry.workgroup_size,
      function,
      workgroup_size_overrides: None,
      mesh_info: entry.mesh_info.take(),
      task_payload: entry.task_payload,
      incoming_ray_payload: None,
    });
  }

  /// Return the outputs at the end of the entry function body, and give the entry function
  /// result. Empty output is possible, for example the depth only render target.
  fn return_entry_outputs(&mut self) -> Option<naga::FunctionResult> {
    if let Some(size) = self.entry.mesh_task_size {
      // task stage must return @builtin(mesh_task_size) vec3<u32> directly,
      // unlike other stages which return a composed output struct
      self.do_return(Some(size));
      let ty = self.register_primitive_ty(PrimitiveShaderValueType::vec3::<u32>());
      Some(naga::FunctionResult {
        ty,
        binding: Some(naga::Binding::BuiltIn(naga::BuiltIn::MeshTaskSize)),
      })
    } else if !self.entry.outputs.is_empty() {
      let ty = ShaderStructMetaInfo {
        name: String::from("ModuleOutput"),
        fields: self.entry.outputs.iter().map(|o| o.meta.clone()).collect(),
        host_layout: None,
      };
      let (ty, _) = gen_struct_define(self, &ty);
      let ty = naga::Type {
        name: None,
        inner: ty,
      };
      let ty = self.module.types.insert(ty, Span::UNDEFINED);

      let output_vars: Vec<_> = self.entry.outputs.iter().map(|o| o.var).collect();
      let components = output_vars
        .into_iter()
        .map(|pointer| self.append_fn_expr(naga::Expression::Load { pointer }))
        .collect();

      let rt = self.make_expression_inner(naga::Expression::Compose { ty, components });
      self.do_return(rt.into());

      Some(naga::FunctionResult { ty, binding: None })
    } else {
      None
    }
  }
}
