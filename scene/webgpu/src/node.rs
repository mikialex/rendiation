use crate::*;

#[pin_project::pin_project]
pub struct SceneNodeGPUSystem {
  #[pin]
  nodes: SceneNodeGPUStorage,
}

impl FusedStream for SceneNodeGPUSystem {
  fn is_terminated(&self) -> bool {
    false
  }
}
impl Stream for SceneNodeGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.nodes.poll_next(cx).map(|v| v.map(|_| {}))
  }
}

pub type ReactiveNodeGPU =
  impl Stream<Item = RenderComponentDeltaFlag> + AsRef<RenderComponentCell<NodeGPU>> + Unpin;

pub type SceneNodeGPUStorage = impl AsRef<StreamVec<ReactiveNodeGPU>>
  + Stream<Item = VecUpdateUnit<RenderComponentDeltaFlag>>
  + Unpin;

impl SceneNodeGPUSystem {
  pub fn new(scene: &SceneCore, derives: &SceneNodeDeriveSystem, cx: &ResourceGPUCtx) -> Self {
    fn build_reactive_node(mat: WorldMatrixStream, cx: &ResourceGPUCtx) -> ReactiveNodeGPU {
      let node = NodeGPU::new(&cx.device);
      let state = RenderComponentCell::new(node);

      let cx = cx.clone();

      mat.fold_signal(state, move |delta, state| {
        state.inner.update(&cx.queue, delta);
        RenderComponentDeltaFlag::Content.into()
      })
    }

    let derives = derives.clone();
    let cx = cx.clone();

    let nodes = scene
      .unbound_listen_by(|view, send| match view {
        MaybeDeltaRef::All(scene) => scene.nodes.expand(send),
        MaybeDeltaRef::Delta(delta) => {
          if let SceneInternalDelta::nodes(node_d) = delta {
            send(node_d.clone())
          }
        }
      })
      .filter_map_sync(move |v| match v {
        tree::TreeMutation::Create { node: idx, .. } => {
          let world_st = derives.create_world_matrix_stream_by_raw_handle(idx)?;
          let node = build_reactive_node(world_st, &cx);
          (idx, node.into()).into()
        }
        tree::TreeMutation::Delete(idx) => (idx, None).into(),
        _ => None,
      })
      .flatten_into_vec_stream_signal();

    Self { nodes }
  }

  pub fn get_node_gpu(&self, node: &SceneNode) -> Option<&NodeGPU> {
    self.get_by_raw(node.raw_handle().index())
  }

  pub fn get_by_raw(&self, index: usize) -> Option<&NodeGPU> {
    self.nodes.as_ref().get(index).map(|v| &v.as_ref().inner)
  }
}

pub struct NodeGPU {
  pub ubo: UniformBufferCachedDataView<TransformGPUData>,
}

impl NodeGPU {
  pub fn update(&mut self, queue: &GPUQueue, world_mat: Mat4<f32>) -> &mut Self {
    self.ubo.set(TransformGPUData::from_world_mat(world_mat));
    self.ubo.upload_with_diff(queue);
    self
  }

  pub fn new(device: &GPUDevice) -> Self {
    let ubo = create_uniform_with_cache(TransformGPUData::default(), device);
    Self { ubo }
  }

  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> BindingPreparer<ShaderUniformPtr<TransformGPUData>> {
    builder
      .bind_by(&self.ubo)
      .using_graphics_pair(builder, |r, node| {
        let node = node.load().expand();
        r.register_typed_both_stage::<WorldMatrix>(node.world_matrix);
        r.register_typed_both_stage::<WorldNormalMatrix>(node.normal_matrix);
      })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, PartialEq, ShaderStruct)]
pub struct TransformGPUData {
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Shader16PaddedMat3,
}

impl TransformGPUData {
  pub fn from_world_mat(world_matrix: Mat4<f32>) -> Self {
    Self {
      world_matrix,
      normal_matrix: world_matrix.to_normal_matrix().into(),
      ..Zeroable::zeroed()
    }
  }
}

impl ShaderHashProvider for NodeGPU {}

impl GraphicsShaderProvider for NodeGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, binding| {
      let model = binding.bind_by(&self.ubo).load().expand();
      let position = builder.query::<GeometryPosition>()?;
      let position = model.world_matrix * (position, val(1.)).into();

      builder.register::<WorldMatrix>(model.world_matrix);
      builder.register::<WorldNormalMatrix>(model.normal_matrix);
      builder.register::<WorldVertexPosition>(position.xyz());

      let normal = builder.query::<GeometryNormal>()?;
      builder.register::<WorldVertexNormal>(model.normal_matrix * normal);
      Ok(())
    })
  }
}

impl ShaderPassBuilder for NodeGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.ubo);
  }
}
