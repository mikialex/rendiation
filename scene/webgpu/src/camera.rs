use crate::*;

#[pin_project::pin_project]
pub struct SceneCameraGPUSystem {
  cameras: SceneCameraGPUStorage,
}

impl Stream for SceneCameraGPUSystem {
  type Item = Vec<StreamMapDelta<usize, CameraGPUTransform>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let r = this.cameras.poll_next_unpin(cx);

    r.map(|v| {
      v.map(|vs| {
        vs.into_iter()
          .map(|v| {
            v.map(|k, _| {
              this
                .cameras
                .as_ref()
                .get(k)
                .unwrap()
                .as_ref()
                .inner
                .ubo
                .get()
            })
          })
          .collect()
      })
    })
  }
}

pub type ReactiveCameraGPU = impl Stream<Item = RenderComponentDeltaFlag>
  + AsRef<RenderComponentCell<CameraGPU>>
  + AsMut<RenderComponentCell<CameraGPU>>
  + Unpin;

pub type SceneCameraGPUStorage = impl AsRef<StreamMap<usize, ReactiveCameraGPU>>
  + AsMut<StreamMap<usize, ReactiveCameraGPU>>
  + Stream<Item = Vec<StreamMapDelta<usize, RenderComponentDeltaFlag>>>
  + Unpin;

enum CameraGPUDelta {
  Proj(Mat4<f32>),
  WorldMat(Mat4<f32>),
  // Jitter(Vec2<f32>),
  // JitterEnable(bool),
}

pub fn build_reactive_camera(
  camera: SceneCamera,
  derives: &SceneNodeDeriveSystem,
  cx: &ResourceGPUCtx,
) -> ReactiveCameraGPU {
  let cx = cx.clone();
  let derives = derives.clone();

  let camera_world = camera
    .single_listen_by(with_field!(SceneCameraInner => node))
    .filter_map_sync(move |node| derives.create_world_matrix_stream(&node))
    .flatten_signal()
    .map(CameraGPUDelta::WorldMat);

  let camera_proj = camera
    .create_projection_mat_stream()
    .map(CameraGPUDelta::Proj);

  let camera = CameraGPU::new(&cx.device);
  let state = RenderComponentCell::new(camera);

  futures::stream::select(camera_world, camera_proj).fold_signal(state, move |delta, state| {
    let uniform = &mut state.inner.ubo;
    uniform.mutate(|uniform| match delta {
      CameraGPUDelta::Proj(proj) => {
        uniform.projection = proj;
        uniform.projection_inv = proj.inverse_or_identity();
        uniform.view_projection = proj * uniform.view;
        uniform.view_projection_inv = uniform.view_projection.inverse_or_identity();
      }
      CameraGPUDelta::WorldMat(world) => {
        uniform.world = world;
        uniform.view = world.inverse_or_identity();
        uniform.rotation = world.extract_rotation_mat();
        uniform.view_projection = uniform.projection * uniform.view;
        uniform.view_projection_inv = uniform.view_projection.inverse_or_identity();
      }
    });

    uniform.upload(&cx.queue);
    RenderComponentDeltaFlag::Content.into()
  })
}

impl SceneCameraGPUSystem {
  pub fn get_camera_gpu(&self, camera: &SceneCamera) -> Option<&CameraGPU> {
    self
      .cameras
      .as_ref()
      .get(&camera.guid())
      .map(|v| &v.as_ref().inner)
  }

  pub fn get_camera_gpu_mut(&mut self, camera: &SceneCamera) -> Option<&mut CameraGPU> {
    self
      .cameras
      .as_mut()
      .get_mut(&camera.guid())
      .map(|v| &mut v.as_mut().inner)
  }

  pub fn get_or_insert(
    &mut self,
    camera: &SceneCamera,
    derives: &SceneNodeDeriveSystem,
    cx: &ResourceGPUCtx,
  ) -> &mut ReactiveCameraGPU {
    self.cameras.as_mut().get_or_insert_with(camera.guid(), || {
      build_reactive_camera(camera.clone(), derives, cx)
    })
  }

  pub fn new(scene: &SceneCore, derives: &SceneNodeDeriveSystem, cx: &ResourceGPUCtx) -> Self {
    let derives = derives.clone();
    let cx = cx.clone();

    let mut index_mapper = FastHashMap::<SceneCameraHandle, usize>::default();

    let cameras = scene
      .unbound_listen_by(with_field_expand!(SceneCoreImpl => cameras))
      .map(move |v: arena::ArenaDelta<SceneCamera>| match v {
        arena::ArenaDelta::Mutate((camera, idx)) => {
          index_mapper.remove(&idx).unwrap();
          index_mapper.insert(idx, camera.guid());
          (
            camera.guid(),
            build_reactive_camera(camera, &derives, &cx).into(),
          )
        }
        arena::ArenaDelta::Insert((camera, idx)) => {
          index_mapper.insert(idx, camera.guid());
          (
            camera.guid(),
            build_reactive_camera(camera, &derives, &cx).into(),
          )
        }
        arena::ArenaDelta::Remove(idx) => {
          let id = index_mapper.remove(&idx).unwrap();
          (id, None)
        }
      })
      .flatten_into_map_stream_signal();

    Self { cameras }
  }
}

pub struct CameraGPU {
  pub enable_jitter: bool,
  pub ubo: UniformBufferDataView<CameraGPUTransform>,
}

impl CameraGPU {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> BindingPreparer<CameraGPUTransform> {
    builder.bind_by(&self.ubo).using_both(builder, |r, camera| {
      let camera = camera.expand();
      r.register_typed_both_stage::<CameraViewMatrix>(camera.view);
      r.register_typed_both_stage::<CameraProjectionMatrix>(camera.projection);
      r.register_typed_both_stage::<CameraProjectionInverseMatrix>(camera.projection_inv);
      r.register_typed_both_stage::<CameraWorldMatrix>(camera.world);
      r.register_typed_both_stage::<CameraViewProjectionMatrix>(camera.view_projection);
      r.register_typed_both_stage::<CameraViewProjectionInverseMatrix>(camera.view_projection_inv);
    })
  }

  pub fn new(device: &GPUDevice) -> Self {
    Self {
      enable_jitter: false,
      ubo: create_uniform(CameraGPUTransform::default(), device),
    }
  }
}

impl ShaderHashProvider for CameraGPU {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.enable_jitter.hash(hasher)
  }
}

impl ShaderPassBuilder for CameraGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.ubo)
  }
}

impl GraphicsShaderProvider for CameraGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    let camera = self.inject_uniforms(builder);

    builder.vertex(|builder, _| {
      let camera = camera.using().expand();
      let position = builder.query::<WorldVertexPosition>()?;

      let mut clip_position = camera.view_projection * (position, val(1.)).into();

      if self.enable_jitter {
        let jitter = if let Ok(texel_size) = builder.query::<TexelSize>() {
          let jitter = texel_size * camera.jitter_normalized * clip_position.w();
          (jitter, val(0.), val(0.)).into()
        } else {
          Vec4::zero().into()
        };
        clip_position += jitter;
      }

      builder.register::<ClipPosition>(clip_position);

      Ok(())
    })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct CameraGPUTransform {
  pub projection: Mat4<f32>,
  pub projection_inv: Mat4<f32>,

  pub rotation: Mat4<f32>,

  pub view: Mat4<f32>,
  pub world: Mat4<f32>,

  pub view_projection: Mat4<f32>,
  pub view_projection_inv: Mat4<f32>,

  /// -0.5 to 0.5
  pub jitter_normalized: Vec2<f32>,
}

pub fn shader_uv_space_to_world_space(
  camera: &ENode<CameraGPUTransform>,
  uv: Node<Vec2<f32>>,
  ndc_depth: Node<f32>,
) -> Node<Vec3<f32>> {
  let xy = uv * val(2.) - val(Vec2::one());
  let xy = xy * val(Vec2::new(1., -1.));
  let ndc = (xy, ndc_depth, val(1.)).into();
  let world = camera.view_projection_inv * ndc;
  world.xyz() / world.w().splat()
}

pub fn shader_world_space_to_uv_space(
  camera: &ENode<CameraGPUTransform>,
  world: Node<Vec3<f32>>,
) -> (Node<Vec2<f32>>, Node<f32>) {
  let clip = camera.view_projection * (world, val(1.)).into();
  let ndc = clip.xyz() / clip.w().splat();
  let uv = ndc.xy() * val(Vec2::new(0.5, -0.5)) + val(Vec2::splat(0.5));
  (uv, ndc.z())
}

pub fn setup_viewport(cb: &CameraViewBounds, pass: &mut GPURenderPass, buffer_size: Size) {
  let width: usize = buffer_size.width.into();
  let width = width as f32;
  let height: usize = buffer_size.height.into();
  let height = height as f32;
  pass.set_viewport(
    width * cb.to_left,
    height * cb.to_top,
    width * cb.width,
    height * cb.height,
    0.,
    1.,
  )
}
