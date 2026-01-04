use interning::*;

use crate::*;

pub struct StateIntern;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for StateIntern {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = InternedId<RasterizationStates>>;

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let (cx, intern) = cx.use_sharable_plain_state(ValueInterning::default);

    cx.use_dual_query::<StandardModelRasterizationOverride>()
      .dual_query_filter_map(|v| v) // todo, we should use prefilter if state setting is parse(likely)
      .use_dual_query_execute_map(cx, move || {
        let mut intern = intern.make_write_holder();
        move |_, v| intern.compute_intern_id(&v)
      })
  }
}

pub fn use_state_overrides(cx: &mut QueryGPUHookCx, reverse_z: bool) -> Option<StateOverrides> {
  let interned = cx
    .use_shared_dual_query(StateIntern)
    .dual_query_boxed()
    .use_assure_result(cx);

  cx.when_render(|| StateOverrides {
    states: read_global_db_component(),
    interned: interned.expect_resolve_stage().view,
    reverse_z,
  })
}

pub struct StateOverrides {
  states: ComponentReadView<StandardModelRasterizationOverride>,
  interned: BoxedDynQuery<RawEntityHandle, InternedId<RasterizationStates>>,
  reverse_z: bool,
}

impl StateOverrides {
  pub fn get_gpu(&self, id: EntityHandle<StandardModelEntity>) -> Option<StateGPUImpl<'_>> {
    let states = self.states.get(id)?;
    let id = self.interned.access(&id.into_raw());

    Some(StateGPUImpl {
      state_id: id,
      states,
      is_reverse_z: self.reverse_z,
    })
  }
}

pub struct StateGPUImpl<'a> {
  state_id: Option<InternedId<RasterizationStates>>,
  states: &'a Option<RasterizationStates>,
  is_reverse_z: bool,
}

impl<'a> ShaderHashProvider for StateGPUImpl<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.state_id.hash(hasher);
  }
  shader_hash_type_id! {StateGPUImpl<'static>}
}

impl<'a> ShaderPassBuilder for StateGPUImpl<'a> {}

impl<'a> GraphicsShaderProvider for StateGPUImpl<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    if let Some(state) = &self.states {
      builder.vertex(|builder, _| {
        builder.primitive_state.front_face = state.front_face;
        builder.primitive_state.cull_mode = state.cull_mode;
      });

      builder.fragment(|builder, _| {
        apply_pipeline_builder(state, self.is_reverse_z, builder);
      })
    }
  }
}

fn map_color_states(states: &RasterizationStates, format: TextureFormat) -> ColorTargetState {
  let mut s = ColorTargetState {
    format,
    blend: states.blend,
    write_mask: states.write_mask,
  };

  if !is_texture_fmt_blendable(format) {
    s.blend = None;
  }

  s
}
fn map_depth_stencil_state(
  states: &RasterizationStates,
  format: Option<TextureFormat>,
  reverse_z: bool,
) -> Option<DepthStencilState> {
  format.map(|format| DepthStencilState {
    format,
    depth_write_enabled: states.depth_write_enabled,
    depth_compare: match states.depth_compare {
      SemanticCompareFunction::Never => CompareFunction::Never,
      SemanticCompareFunction::Nearer => {
        if reverse_z {
          CompareFunction::Greater
        } else {
          CompareFunction::Less
        }
      }
      SemanticCompareFunction::Equal => CompareFunction::Equal,
      SemanticCompareFunction::NearerEqual => {
        if reverse_z {
          CompareFunction::GreaterEqual
        } else {
          CompareFunction::LessEqual
        }
      }
      SemanticCompareFunction::Further => {
        if reverse_z {
          CompareFunction::Less
        } else {
          CompareFunction::Greater
        }
      }
      SemanticCompareFunction::NotEqual => CompareFunction::NotEqual,
      SemanticCompareFunction::FurtherEqual => {
        if reverse_z {
          CompareFunction::LessEqual
        } else {
          CompareFunction::GreaterEqual
        }
      }
      SemanticCompareFunction::Always => CompareFunction::Always,
    },
    stencil: states.stencil.clone(),
    bias: states.bias,
  })
}

pub fn apply_pipeline_builder(
  states: &RasterizationStates,
  reverse_z: bool,
  builder: &mut ShaderFragmentBuilder,
) {
  // override all outputs states
  builder.frag_output.iter_mut().for_each(|p| {
    let format = p.states.format;
    p.states = map_color_states(states, format);
  });

  // and depth_stencil if they exist
  let format = builder.depth_stencil.as_ref().map(|s| s.format);
  builder.depth_stencil = map_depth_stencil_state(states, format, reverse_z);
}
