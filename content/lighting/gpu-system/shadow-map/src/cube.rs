use database::RawEntityHandle;
use fast_hash_collection::FastHashMap;
use rendiation_texture_packer::pack_2d_to_3d::RemappedGrowablePacker;

use crate::*;

pub const CUBE_FACE_COUNT: usize = 6;

pub struct CubeShadowMapInfoInput {
  pub light_world: Mat4<f64>,
  pub proj: ShadowCameraProjectionMatrixes,
  /// the size of one cube face
  pub map_size: Size,
  pub bias: ShadowBias,
}

/// the six faces of one light are packed in a 2 x 3 grid
fn cube_atlas_region_size(map_size: Size) -> Size {
  Size::from_usize_pair_min_one((map_size.width_usize() * 2, map_size.height_usize() * 3))
}

/// the face direction convention follows the CubeTextureFace order
fn cube_face_direction(face: usize) -> (Vec3<f64>, Vec3<f64>) {
  match face {
    0 => (Vec3::new(1., 0., 0.), Vec3::new(0., 1., 0.)),
    1 => (Vec3::new(-1., 0., 0.), Vec3::new(0., 1., 0.)),
    2 => (Vec3::new(0., 1., 0.), Vec3::new(0., 0., -1.)),
    3 => (Vec3::new(0., -1., 0.), Vec3::new(0., 0., 1.)),
    4 => (Vec3::new(0., 0., 1.), Vec3::new(0., 1., 0.)),
    _ => (Vec3::new(0., 0., -1.), Vec3::new(0., 1., 0.)),
  }
}

/// the returned matrix follows the engine camera world matrix convention
/// (same as Mat4::lookat), the forward row is the reverse of the view
/// direction so the camera space forward is -z
pub fn build_cube_face_world_matrices(light_world: Mat4<f64>) -> [Mat4<f64>; CUBE_FACE_COUNT] {
  let position = light_world.position();
  std::array::from_fn(|face| {
    let (forward, up) = cube_face_direction(face);
    Mat4::lookat(position, position + forward, up)
  })
}

fn build_cube_face_infos(
  pack: PackResult2dWithDepth,
  map_size: Size,
  proj: &ShadowCameraProjectionMatrixes,
  face_worlds: &[Mat4<f64>; CUBE_FACE_COUNT],
) -> [CubeFaceShadowMapInfo; CUBE_FACE_COUNT] {
  let origin = pack.result.range.origin;
  let face_w = map_size.width_usize();
  let face_h = map_size.height_usize();

  std::array::from_fn(|face| {
    let col = face % 2;
    let row = face / 2;

    let map_info = ShadowMapAddressInfo {
      layer_index: pack.depth as i32,
      size: Vec2::new(face_w as f32, face_h as f32),
      offset: Vec2::new(
        (origin.x + col * face_w) as f32,
        (origin.y + row * face_h) as f32,
      ),
      ..Default::default()
    };

    let world_inv = face_worlds[face].inverse_or_identity();
    CubeFaceShadowMapInfo {
      map_info,
      shadow_center_without_translation_to_shadowmap_ndc: proj.render_matrix
        * world_inv.remove_position().into_f32(),
      proj_linear_depth_recover_helper: extract_shadow_proj_linear_depth_recover_helper(
        proj.opengl_ndc_matrix,
      ),
      ..Default::default()
    }
  })
}

#[derive(Clone, Default)]
pub struct CubeShadowMapInfoGPU {
  // scene entity -> per-scene uniform buffer
  pub uniforms: FastHashMap<RawEntityHandle, UniformArray<CubeShadowMapInfo, MAX_SHADOW_COUNT>>,
}

/// shadow_info_access: light_id -> Option<CubeShadowMapInfoInput>, return None if no shadow
///
/// return (preparer, shadow_map_atlas_size_require)
pub fn prepare_cube_shadow_map_uniform(
  atlas_config: &MultiLayerTexturePackerConfig,
  light_uniform_array_index_mapping: &LightArrayAllocateResult,
  shadow_info_access: &dyn Fn(RawEntityHandle) -> Option<CubeShadowMapInfoInput>,
  gpu_data: &mut Option<CubeShadowMapInfoGPU>,
  gpu: &GPU,
) -> (CubeShadowMapPreparer, SizeWithDepth) {
  let mut packer = RemappedGrowablePacker::<RawEntityHandle>::new(*atlas_config);
  let mut light_worlds = FastHashMap::default();
  let mut light_proj_infos = FastHashMap::default();
  let mut light_face_infos = FastHashMap::default();

  let new_shadow_info: FastHashMap<
    RawEntityHandle,
    Shader140Array<CubeShadowMapInfo, MAX_SHADOW_COUNT>,
  > = light_uniform_array_index_mapping
    .iter()
    .map(|(scene_id, light_id_mapping)| {
      let mut shadow_info_array = Shader140Array::<CubeShadowMapInfo, MAX_SHADOW_COUNT>::default();

      // packer maybe resize, so we have to batch process first
      let sizes = light_id_mapping.iter().filter_map(|(light_id, _)| {
        shadow_info_access(*light_id).map(|v| (*light_id, cube_atlas_region_size(v.map_size)))
      });
      packer.process([].into_iter(), sizes, |_| {}, |_, _| {});

      for (light_id, uniform_array_index) in light_id_mapping.iter() {
        let shadow_uniform = if let Some(shadow_info) = shadow_info_access(*light_id) {
          // todo, handle allocation fail(warning and handle shader access)
          let face_worlds = build_cube_face_world_matrices(shadow_info.light_world);

          let faces = packer
            .access(light_id)
            .unwrap()
            .map(|pack| {
              build_cube_face_infos(pack, shadow_info.map_size, &shadow_info.proj, &face_worlds)
            })
            .unwrap_or_default();

          light_worlds.insert(*light_id, face_worlds);
          light_proj_infos.insert(*light_id, shadow_info.proj);
          light_face_infos.insert(*light_id, faces);

          let shadow_world_position = into_hpt(shadow_info.light_world.position()).into_uniform();

          CubeShadowMapInfo {
            enabled: Bool::from(true),
            shadow_world_position,
            bias: shadow_info.bias,
            faces: faces.into(),
            ..Default::default()
          }
        } else {
          CubeShadowMapInfo {
            enabled: Bool::from(false),
            ..Default::default()
          }
        };
        shadow_info_array.set(*uniform_array_index as usize, shadow_uniform);
      }
      (*scene_id, shadow_info_array)
    })
    .collect();

  let uniforms = gpu_data.get_or_insert_default();

  let uniforms: FastHashMap<RawEntityHandle, UniformArray<CubeShadowMapInfo, MAX_SHADOW_COUNT>> =
    new_shadow_info
      .iter()
      .map(|(scene_id, info)| {
        let uniform = if let Some(existing) = uniforms.uniforms.remove(scene_id) {
          existing.write_at(&gpu.queue, info, 0);
          existing
        } else {
          create_uniform(*info, &gpu.device, "cube-shadow-map-uniform")
        };
        (*scene_id, uniform)
      })
      .collect();

  *gpu_data = Some(CubeShadowMapInfoGPU {
    uniforms: uniforms.clone(),
  });

  let required_size = packer.current_size();

  (
    CubeShadowMapPreparer {
      gpu_data: CubeShadowMapInfoGPU { uniforms },
      light_worlds,
      light_proj_infos,
      light_face_infos,
    },
    required_size,
  )
}

pub struct CubeShadowMapPreparer {
  pub gpu_data: CubeShadowMapInfoGPU,
  // light entity -> per-face camera world
  light_worlds: FastHashMap<RawEntityHandle, [Mat4<f64>; CUBE_FACE_COUNT]>,
  light_proj_infos: FastHashMap<RawEntityHandle, ShadowCameraProjectionMatrixes>,
  // light entity -> per-face info (contains atlas address)
  light_face_infos: FastHashMap<RawEntityHandle, [CubeFaceShadowMapInfo; CUBE_FACE_COUNT]>,
}

impl CubeShadowMapPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    shadow_map: &mut dyn AbstractShadowMapGPUData,
    scene_content: &mut dyn FnMut(&mut FrameCtx, ShadowMapDrawRequest),
  ) -> CubeShadowMapInfoGPU {
    shadow_map.clear_shadow_map(frame_ctx);

    // do shadowmap updates
    for (light_id, faces) in self.light_face_infos.iter() {
      let shadow_camera_proj = self.light_proj_infos.get(light_id).unwrap();
      let shadow_camera_worlds = self.light_worlds.get(light_id).unwrap();

      for (face, face_info) in faces.iter().enumerate() {
        let request = ShadowMapUpdateRequest {
          shadow_camera_proj: *shadow_camera_proj,
          shadow_camera_world: shadow_camera_worlds[face],
          light_id: *light_id,
          address: face_info.map_info,
        };

        // todo, consider merge the pass within the same layer
        shadow_map.update_shadow_map(frame_ctx, request, scene_content);
      }
    }

    self.gpu_data
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct CubeShadowMapInfo {
  pub enabled: Bool,
  pub shadow_world_position: HighPrecisionTranslationUniform,
  pub bias: ShadowBias,
  pub faces: Shader140Array<CubeFaceShadowMapInfo, CUBE_FACE_COUNT>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct CubeFaceShadowMapInfo {
  pub map_info: ShadowMapAddressInfo,
  pub shadow_center_without_translation_to_shadowmap_ndc: Mat4<f32>,
  pub proj_linear_depth_recover_helper: ProjLinearDepthRecoverHelper,
}

#[derive(Clone)]
pub struct CubeShadowMapComponent {
  pub info: UniformBufferDataView<Shader140Array<CubeShadowMapInfo, MAX_SHADOW_COUNT>>,
  pub bias_behavior: ShadowBiasBehaviorConfig,
  pub reversed_depth: bool,
  pub shadow_computer: Arc<dyn AbstractShadowComputer>,
}

impl ShaderHashProvider for CubeShadowMapComponent {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.reversed_depth);
    hasher.hash(self.bias_behavior);
    self.shadow_computer.hash_pipeline(hasher);
  }
}

impl AbstractShaderBindingSource for CubeShadowMapComponent {
  type ShaderBindResult = CubeShadowMapInvocation;
  fn bind_shader(&self, cx: &mut ShaderBindGroupBuilder) -> CubeShadowMapInvocation {
    CubeShadowMapInvocation {
      shadow_computer: self.shadow_computer.bind_shader(cx),
      info: cx.bind_by(&self.info),
      bias_behavior: self.bias_behavior,
      reversed_depth: self.reversed_depth,
    }
  }
}

impl AbstractBindingSource for CubeShadowMapComponent {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    self.shadow_computer.bind_pass(ctx);
    ctx.bind(&self.info);
  }
}

pub struct CubeShadowMapInvocation {
  shadow_computer: Box<dyn AbstractShadowComputerInvocation>,
  info: ShaderReadonlyPtrOf<Shader140Array<CubeShadowMapInfo, MAX_SHADOW_COUNT>>,
  bias_behavior: ShadowBiasBehaviorConfig,
  reversed_depth: bool,
}

impl CubeShadowMapInvocation {
  pub fn query_shadow_occlusion_by_idx(
    &self,
    render_position: Node<Vec3<f32>>,
    render_normal: Node<Vec3<f32>>,
    shadow_idx: Node<u32>,
    screen_position: Node<Vec2<f32>>,
    camera_world_position: Node<HighPrecisionTranslation>,
  ) -> Node<f32> {
    let enabled = self.info.index(shadow_idx).enabled().load();
    enabled.into_bool().select_branched(
      || {
        let shadow_info = self.info.index(shadow_idx);
        let shadow_world_position = shadow_info.shadow_world_position().load();

        // the direction from the light to the shading point selects the face
        let light_position_in_render = hpt_sub_hpt(
          hpt_uniform_to_hpt(shadow_world_position),
          camera_world_position,
        );
        let direction_to_fragment = render_position - light_position_in_render;
        let face_index = select_cube_face_fn(direction_to_fragment);

        let face_info = shadow_info.faces().index(face_index);
        let map_info = face_info.map_info().load();

        let shadow_position = compute_shadow_position(
          render_position,
          render_normal,
          shadow_world_position,
          camera_world_position,
          shadow_info.bias().load(),
          map_info,
          face_info
            .shadow_center_without_translation_to_shadowmap_ndc()
            .load(),
          self.bias_behavior.use_n_dot_l_normal_offset,
          self.reversed_depth,
        );

        self.shadow_computer.compute_shadow(
          shadow_position,
          screen_position,
          map_info,
          val(1.),
          face_info.proj_linear_depth_recover_helper(),
        )
      },
      || val(1.),
    )
  }
}

/// select the cube face index of the direction, the index convention
/// follows the CubeTextureFace order
#[shader_fn]
pub fn select_cube_face(direction: Node<Vec3<f32>>) -> Node<u32> {
  let abs_direction = direction.abs();
  let face_index = zeroed_val::<u32>().make_local_var();

  if_by(
    abs_direction
      .x()
      .greater_than(abs_direction.y())
      .and(abs_direction.x().greater_than(abs_direction.z())),
    || {
      face_index.store(
        direction
          .x()
          .greater_than(val(0.))
          .select(val(0_u32), val(1_u32)),
      );
    },
  )
  .else_if(abs_direction.y().greater_than(abs_direction.z()), || {
    face_index.store(
      direction
        .y()
        .greater_than(val(0.))
        .select(val(2_u32), val(3_u32)),
    );
  })
  .else_by(|| {
    face_index.store(
      direction
        .z()
        .greater_than(val(0.))
        .select(val(4_u32), val(5_u32)),
    );
  });

  face_index.load()
}
