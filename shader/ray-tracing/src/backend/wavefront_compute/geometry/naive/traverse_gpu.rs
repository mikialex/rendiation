use crate::backend::wavefront_compute::geometry::naive::*;

#[derive(Clone)]
pub(super) struct NaiveSahBvhGpu {
  // maps tlas_idx to tlas_handle: tlas_bvh_root[tlas_binding[tlas_idx]]
  pub(super) tlas_binding: StorageBufferReadOnlyDataView<[u32]>,

  // maps user tlas_id to tlas_bvh root node idx in tlas_bvh_forest
  pub(super) tlas_bvh_root: StorageBufferReadOnlyDataView<[u32]>,
  // global bvh, root at tlas_bvh_root[tlas_idx], content_range to index tlas_data/tlas_bounding
  pub(super) tlas_bvh_forest: StorageBufferReadOnlyDataView<[DeviceBVHNode]>,
  // acceleration_structure_handle to index blas_meta_info
  pub(super) tlas_data:
    StorageBufferReadOnlyDataView<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  pub(super) tlas_bounding: StorageBufferReadOnlyDataView<[TlasBounding]>,

  // tri_range to index tri_bvh_root, box_range to index box_bvh_root
  pub(super) blas_meta_info: StorageBufferReadOnlyDataView<[BlasMetaInfo]>,
  // tri_bvh_forest root_idx, geometry_idx, primitive_start, geometry_flags
  pub(super) tri_bvh_root: StorageBufferReadOnlyDataView<[GeometryMetaInfo]>,
  // // box_bvh_forest root_idx, geometry_idx, primitive_start, geometry_flags
  // pub(super) box_bvh_root: StorageBufferReadOnlyDataView<[GeometryMetaInfo]>,
  // content range to index indices
  pub(super) tri_bvh_forest: StorageBufferReadOnlyDataView<[DeviceBVHNode]>,
  // // content range to index boxes
  // pub(super) box_bvh_forest: StorageBufferReadOnlyDataView<[DeviceBVHNode]>,
  pub(super) indices_redirect: StorageBufferReadOnlyDataView<[u32]>,
  pub(super) indices: StorageBufferReadOnlyDataView<[u32]>,
  pub(super) vertices: StorageBufferReadOnlyDataView<[f32]>,
  // pub(super) boxes: StorageBufferReadOnlyDataView<[f32]>,
}

impl GPUAccelerationStructureSystemCompImplInstance for NaiveSahBvhGpu {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureSystemCompImplInvocationTraversable> {
    let tlas_binding = compute_cx.bind_by(&self.tlas_binding);
    let tlas_bvh_root = compute_cx.bind_by(&self.tlas_bvh_root);
    let tlas_bvh_forest = compute_cx.bind_by(&self.tlas_bvh_forest);
    let tlas_data = compute_cx.bind_by(&self.tlas_data);
    let tlas_bounding = compute_cx.bind_by(&self.tlas_bounding);
    let blas_meta_info = compute_cx.bind_by(&self.blas_meta_info);
    let tri_bvh_root = compute_cx.bind_by(&self.tri_bvh_root);
    // let box_bvh_root = compute_cx.bind_by(&self.box_bvh_root);
    let tri_bvh_forest = compute_cx.bind_by(&self.tri_bvh_forest);
    // let box_bvh_forest = compute_cx.bind_by(&self.box_bvh_forest);
    let indices_redirect = compute_cx.bind_by(&self.indices_redirect);
    let indices = compute_cx.bind_by(&self.indices);
    let vertices = compute_cx.bind_by(&self.vertices);
    // let boxes = compute_cx.bind_by(&self.boxes);

    let instance = NaiveSahBVHInvocationInstance {
      tlas_binding,
      tlas_bvh_root,
      tlas_bvh_forest,
      tlas_data,
      tlas_bounding,
      blas_meta_info,
      tri_bvh_root,
      // box_bvh_root,
      tri_bvh_forest,
      // box_bvh_forest,
      indices_redirect,
      indices,
      vertices,
      // boxes,
    };

    Box::new(instance)
  }

  fn bind_pass(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.tlas_binding);
    builder.bind(&self.tlas_bvh_root);
    builder.bind(&self.tlas_bvh_forest);
    builder.bind(&self.tlas_data);
    builder.bind(&self.tlas_bounding);
    builder.bind(&self.blas_meta_info);
    builder.bind(&self.tri_bvh_root);
    // builder.bind(&self.box_bvh_root);
    builder.bind(&self.tri_bvh_forest);
    // builder.bind(&self.box_bvh_forest);
    builder.bind(&self.indices_redirect);
    builder.bind(&self.indices);
    builder.bind(&self.vertices);
    // builder.bind(&self.boxes);
  }

  fn create_tlas_instance(&self) -> Box<dyn GPUAccelerationStructureSystemTlasCompImplInstance> {
    Box::new(NaiveSahBvhGpuTlas {
      tlas_data: self.tlas_data.clone(),
    })
  }
}

pub struct NaiveSahBVHInvocationInstance {
  tlas_binding: ReadOnlyStorageNode<[u32]>,
  tlas_bvh_root: ReadOnlyStorageNode<[u32]>,
  tlas_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  tlas_data: ReadOnlyStorageNode<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  tlas_bounding: ReadOnlyStorageNode<[TlasBounding]>,
  blas_meta_info: ReadOnlyStorageNode<[BlasMetaInfo]>,
  tri_bvh_root: ReadOnlyStorageNode<[GeometryMetaInfo]>,
  // box_bvh_root: ReadOnlyStorageNode<[GeometryMetaInfo]>,
  tri_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  // box_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  indices_redirect: ReadOnlyStorageNode<[u32]>,
  indices: ReadOnlyStorageNode<[u32]>,
  vertices: ReadOnlyStorageNode<[f32]>,
  // boxes: ReadOnlyStorageNode<[f32]>,
}

#[derive(Clone)]
pub(super) struct NaiveSahBvhGpuTlas {
  pub(super) tlas_data:
    StorageBufferReadOnlyDataView<[TopLevelAccelerationStructureSourceDeviceInstance]>,
}
impl GPUAccelerationStructureSystemTlasCompImplInstance for NaiveSahBvhGpuTlas {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureSystemTlasCompImplInvocation> {
    let tlas_data = compute_cx.bind_by(&self.tlas_data);
    Box::new(NaiveSahBVHTlasInvocationInstance { tlas_data })
  }
  fn bind_pass(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.tlas_data);
  }
}
pub struct NaiveSahBVHTlasInvocationInstance {
  tlas_data: ReadOnlyStorageNode<[TopLevelAccelerationStructureSourceDeviceInstance]>,
}
impl GPUAccelerationStructureSystemTlasCompImplInvocation for NaiveSahBVHTlasInvocationInstance {
  fn index_tlas(
    &self,
    idx: Node<u32>,
  ) -> ReadOnlyStorageNode<TopLevelAccelerationStructureSourceDeviceInstance> {
    self.tlas_data.index(idx)
  }
}

struct TraverseBvhIteratorGpu {
  bvh: ReadOnlyStorageNode<[DeviceBVHNode]>,
  ray: Node<Ray>,
  node_idx: LocalVarNode<u32>,
  ray_range: RayRange,
}
impl ShaderIterator for TraverseBvhIteratorGpu {
  type Item = Node<Vec2<u32>>; // node content range
  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let has_next = val(false).make_local_var();
    let item = zeroed_val().make_local_var();

    loop_by(|loop_cx| {
      let idx = self.node_idx.load();
      if_by(idx.equals(val(INVALID_NEXT)), || loop_cx.do_break());
      let node = self.bvh.index(idx).load().expand();
      let (near, far) = self.ray_range.get();
      let hit_aabb = intersect_ray_aabb_gpu(self.ray, node.aabb_min, node.aabb_max, near, far);

      if_by(hit_aabb, || {
        let is_leaf = node.hit_next.equals(node.miss_next);
        self.node_idx.store(node.hit_next);
        if_by(is_leaf, || {
          has_next.store(val(true));
          item.store(node.content_range);
          loop_cx.do_break();
        });
      })
      .else_by(|| {
        self.node_idx.store(node.miss_next);
      });
    });

    (has_next.load(), item.load())
  }
}

/// returns iterator item = tlas_data idx
fn traverse_tlas_gpu(
  root: Node<u32>,
  bvh: ReadOnlyStorageNode<[DeviceBVHNode]>,
  tlas_bounding: ReadOnlyStorageNode<[TlasBounding]>,
  ray: Node<Ray>,
  ray_range: RayRange,
) -> impl ShaderIterator<Item = Node<u32>> {
  let bvh_iter = TraverseBvhIteratorGpu {
    bvh,
    ray,
    node_idx: root.make_local_var(),
    ray_range: ray_range.clone(),
  };
  let iter = bvh_iter.flat_map(ForRange::new);

  iter.filter_map(move |tlas_idx: Node<u32>| {
    let tlas_bounding_pack = tlas_bounding.index(tlas_idx).load();
    let tlas_bounding = tlas_bounding_pack.expand();

    let (near, far) = ray_range.get();
    let hit_tlas = intersect_ray_aabb_gpu(
      ray,
      tlas_bounding.world_min,
      tlas_bounding.world_max,
      near,
      far,
    );

    let ray = ray.expand();
    let pass_mask = ray.mask.bitand(tlas_bounding.mask).not_equals(val(0));

    let hit = hit_tlas.and(pass_mask);

    (hit, tlas_idx)
  })
}

impl GPUAccelerationStructureSystemCompImplInvocationTraversable for NaiveSahBVHInvocationInstance {
  fn traverse(
    &self,
    trace_payload: ENode<ShaderRayTraceCallStoragePayload>,
    user_defined_payloads: StorageNode<[u32]>,
    intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
    any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> ShaderOption<RayClosestHitCtx> {
    let ray = Ray::construct(RayShaderAPIInstance {
      origin: trace_payload.ray_origin,
      flags: trace_payload.ray_flags,
      direction: trace_payload.ray_direction,
      mask: trace_payload.cull_mask,
      // range: trace_payload.range,
    });

    let world_ray_range = RayRange::new(trace_payload.range);

    let tlas_selected = self.tlas_binding.index(trace_payload.tlas_idx).load();
    let tlas_bvh_root = self.tlas_bvh_root.index(tlas_selected).load();

    let tlas_idx_iter = traverse_tlas_gpu(
      tlas_bvh_root, // tlas_bvh_root == INVALID_NEXT checked inside TraverseBvhIteratorGpu
      self.tlas_bvh_forest,
      self.tlas_bounding,
      ray,
      world_ray_range.clone(),
    );

    let blas_iter = iterate_tlas_blas_gpu(tlas_idx_iter, self.tlas_data, self.blas_meta_info, ray);

    // construct ctx
    let launch_info = RayLaunchInfo {
      launch_id: trace_payload.launch_id,
      launch_size: trace_payload.launch_size,
    };
    let world_ray = WorldRayInfo {
      world_ray: ShaderRay {
        origin: trace_payload.ray_origin,
        direction: trace_payload.ray_direction,
      },
      ray_range: ShaderRayRange {
        min: trace_payload.range.x(),
        max: trace_payload.range.y(),
      },
      ray_flags: trace_payload.ray_flags,
    };

    let hit_ctx_info_var = HitCtxInfoVar::default();
    let hit_info_var = HitInfoVar::default();
    hit_info_var.hit_distance.store(world_ray.ray_range.max);

    let user_defined_payload = U32BufferLoadStoreSource {
      array: user_defined_payloads,
      offset: trace_payload.payload_ref,
    };

    intersect_blas_gpu(
      user_defined_payload,
      blas_iter,
      self.tlas_data,
      self.tri_bvh_root,
      self.tri_bvh_forest,
      // self.box_bvh_root,
      // self.box_bvh_forest,
      self.indices_redirect,
      self.indices,
      self.vertices,
      // self.boxes,
      intersect,
      any_hit,
      launch_info,
      world_ray,
      &hit_ctx_info_var, // output
      &hit_info_var,     // output
      world_ray_range.clone(),
    );

    let hit_ctx_info = hit_ctx_info_var.load(self.tlas_data);
    let hit_info = HitInfo {
      hit_kind: hit_info_var.hit_kind.load(),
      hit_distance: hit_info_var.hit_distance.load(),
      hit_attribute: hit_info_var.hit_attribute.load(),
    };

    ShaderOption {
      is_some: hit_info_var.any_hit.load(),
      payload: RayClosestHitCtx {
        launch_info,
        world_ray,
        hit_ctx: hit_ctx_info,
        hit: hit_info,
      },
    }
  }
}

struct NaiveIntersectReporter<'a> {
  launch_info: RayLaunchInfo,
  world_ray: WorldRayInfo,
  hit_ctx: HitCtxInfo,
  closest_hit_ctx_info: &'a HitCtxInfoVar,
  closest_hit_info: &'a HitInfoVar,
  ray_range: RayRange,
  any_hit: &'a dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  on_end_search: Box<dyn Fn()>,
  user_defined_payload: U32BufferLoadStoreSource,
}
impl IntersectionReporter for NaiveIntersectReporter<'_> {
  fn report_intersection(
    &self,
    hit_t: Node<f32>,
    hit_kind: Node<u32>,
    hit_attribute: Node<HitAttribute>,
  ) -> Node<bool> {
    let r = val(false).make_local_var();
    let (near, far) = self.ray_range.get();

    let in_range = near.less_equal_than(hit_t).and(hit_t.less_equal_than(far));

    if_by(in_range, || {
      let any_hit_ctx = RayAnyHitCtx {
        launch_info: self.launch_info,
        world_ray: self.world_ray,
        hit_ctx: self.hit_ctx,
        hit: HitInfo {
          hit_kind,
          hit_distance: hit_t,
          hit_attribute,
        },
        payload: self.user_defined_payload,
      };
      let closest_hit_ctx = self.closest_hit_ctx_info;
      let closest_hit = self.closest_hit_info;
      let any_hit = self.any_hit;

      resolve_any_hit(
        |ctx| {
          r.store(val(true));
          self.ray_range.update_world_far(ctx.hit.hit_distance);
        },
        || (self.on_end_search)(),
        any_hit,
        &any_hit_ctx,
        closest_hit_ctx,
        closest_hit,
      );
    });
    r.load()
  }
}

fn resolve_any_hit(
  on_accept: impl FnOnce(&RayAnyHitCtx),
  on_end_search: impl FnOnce(),
  any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  any_hit_ctx: &RayAnyHitCtx,
  closest_hit_ctx: &HitCtxInfoVar, // output
  closest_hit: &HitInfoVar,        // output
) {
  let behavior = any_hit(any_hit_ctx);

  if_by(
    (behavior & (val(ANYHIT_BEHAVIOR_ACCEPT_HIT))).greater_than(val(0)),
    || {
      // hit! update closest
      closest_hit.test_and_store(&any_hit_ctx.hit, || {
        closest_hit_ctx.store(&any_hit_ctx.hit_ctx);
        on_accept(any_hit_ctx);
      });
    },
  );

  if_by(
    (behavior & val(ANYHIT_BEHAVIOR_END_SEARCH)).greater_than(0),
    || {
      on_end_search();
    },
  );
}

#[derive(Clone)]
pub(crate) struct RayRange {
  near: Node<f32>,
  far: LocalVarNode<f32>,
  scaling: Option<Node<f32>>,
}
impl RayRange {
  pub fn new(ray_range: Node<Vec2<f32>>) -> Self {
    Self {
      near: ray_range.x(),
      far: ray_range.y().make_local_var(),
      scaling: None,
    }
  }
  pub fn clone_with_scaling(&self, scaling: Node<f32>) -> Self {
    Self {
      near: self.near,
      far: self.far,
      scaling: Some(scaling),
    }
  }

  pub fn update_world_far(&self, far: Node<f32>) {
    self.far.store(far);
  }
  pub fn get(&self) -> (Node<f32>, Node<f32>) {
    if let Some(scaling) = self.scaling {
      (self.near * scaling, self.far.load() * scaling)
    } else {
      (self.near, self.far.load())
    }
  }
}

#[repr(C)]
#[derive(ShaderStruct, Clone, Copy)]
struct RayBlas {
  pub ray: Ray,
  pub blas: BlasMetaInfo,
  pub tlas_idx: u32,
  pub distance_scaling: f32,
  pub flags: u32,
}

fn iterate_tlas_blas_gpu(
  tlas_iter: impl ShaderIterator<Item = Node<u32>>,
  tlas_data: ReadOnlyStorageNode<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  blas_data: ReadOnlyStorageNode<[BlasMetaInfo]>,
  ray: Node<Ray>,
) -> impl ShaderIterator<Item = Node<RayBlas>> {
  tlas_iter.map(move |idx: Node<u32>| {
    let ray = ray.expand();
    let tlas_data = tlas_data.index(idx).load().expand();

    let flags = TraverseFlagsGpu::from_ray_flag(ray.flags);
    let flags = flags.merge_geometry_instance_flag(tlas_data.flags);

    // transform ray to blas space
    let blas_ray_origin = tlas_data.transform_inv * (ray.origin, val(1.)).into();
    let blas_ray_origin = blas_ray_origin.xyz() / blas_ray_origin.w().splat();
    let blas_ray_direction = tlas_data.transform_inv.shrink_to_3() * ray.direction;
    let distance_scaling = blas_ray_direction.length();
    // let blas_ray_range = ray_range.clone_with_scaling(distance_scaling);
    let blas_ray_direction = blas_ray_direction.normalize();

    let blas_ray = Ray::construct(RayShaderAPIInstance {
      origin: blas_ray_origin,
      flags: ray.flags,
      direction: blas_ray_direction,
      mask: ray.mask,
      // range: val(vec2(0., 0.)), // not used, calculated from
    });

    let blas_idx = tlas_data.acceleration_structure_handle;
    let blas_data = blas_data.index(blas_idx).load();

    RayBlas::construct(RayBlasShaderAPIInstance {
      ray: blas_ray,
      blas: blas_data,
      tlas_idx: idx,
      distance_scaling,
      flags: flags.as_u32(),
    })
  })
}

fn intersect_blas_gpu(
  user_defined_payload: U32BufferLoadStoreSource,
  blas_iter: impl ShaderIterator<Item = Node<RayBlas>>,
  tlas_data: ReadOnlyStorageNode<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  tri_bvh_root: ReadOnlyStorageNode<[GeometryMetaInfo]>,
  tri_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  // _box_bvh_root: ReadOnlyStorageNode<[GeometryMetaInfo]>,
  // _box_bvh_forest: ReadOnlyStorageNode<[DeviceBVHNode]>,
  indices_redirect: ReadOnlyStorageNode<[u32]>,
  indices: ReadOnlyStorageNode<[u32]>,
  vertices: ReadOnlyStorageNode<[f32]>,
  // _boxes: ReadOnlyStorageNode<[f32]>,
  intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
  any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,

  launch_info: RayLaunchInfo,
  world_ray: WorldRayInfo,
  closest_hit_ctx_var: &HitCtxInfoVar, // output
  closest_hit_var: &HitInfoVar,        // output

  world_ray_range: RayRange, // input/output
) {
  let hit_ctx_curr = HitCtxInfoVar::default();
  let end_search = val(false).make_local_var();

  blas_iter.for_each(|ray_blas, blas_loop| {
    let ray_blas = ray_blas.expand();
    let ray = ray_blas.ray;
    let blas = ray_blas.blas.expand();
    let flags = TraverseFlagsGpu::from_ray_flag(ray_blas.flags);
    let distance_scaling = ray_blas.distance_scaling;
    let local_ray_range = world_ray_range.clone_with_scaling(distance_scaling);

    ForRange::new(blas.tri_root_range).for_each(move |tri_root_idx, mesh_loop| {
      let geometry = tri_bvh_root.index(tri_root_idx).load().expand();
      let root = geometry.bvh_root_idx;
      let geometry_id = geometry.geometry_idx;
      let primitive_start = geometry.primitive_start;
      let geometry_flags = geometry.geometry_flags;

      let (pass, is_opaque) = flags.cull_geometry(geometry_flags);
      if_by(pass.not(), || {
        mesh_loop.do_continue();
      });
      let (cull_enable, cull_back) = flags.cull_triangle();

      let local_ray_range = local_ray_range.clone();
      if_by(flags.visit_triangles(), move || {
        let bvh_iter = TraverseBvhIteratorGpu {
          bvh: tri_bvh_forest,
          ray,
          node_idx: root.make_local_var(),
          ray_range: local_ray_range.clone(),
        };
        let tri_idx_iter = bvh_iter.flat_map(ForRange::new); // triangle index

        let ray = ray.expand();

        fn read_vec3<T: ShaderNodeType>(
          idx: Node<u32>,
          array: ReadOnlyStorageNode<[T]>,
        ) -> [Node<T>; 3] {
          let i = idx * val(3);
          let v0 = array.index(i).load();
          let v1 = array.index(i + val(1)).load();
          let v2 = array.index(i + val(2)).load();
          [v0, v1, v2]
        }

        tri_idx_iter.for_each(move |tri_idx, tri_loop| {
          let tri_idx = indices_redirect.index(tri_idx).load();
          let [i0, i1, i2] = read_vec3(tri_idx, indices);
          let [v0x, v0y, v0z] = read_vec3(i0, vertices);
          let [v1x, v1y, v1z] = read_vec3(i1, vertices);
          let [v2x, v2y, v2z] = read_vec3(i2, vertices);
          let v0 = Node::<Vec3<f32>>::from((v0x, v0y, v0z));
          let v1 = Node::<Vec3<f32>>::from((v1x, v1y, v1z));
          let v2 = Node::<Vec3<f32>>::from((v2x, v2y, v2z));

          let (near, far) = local_ray_range.get();
          // returns (hit, distance, u, v), hit = front hit -> 1, back hit -> -1, miss -> 0
          let result = intersect_ray_triangle_gpu(
            ray.origin,
            ray.direction,
            near,
            far,
            v0,
            v1,
            v2,
            cull_enable,
            cull_back,
          );
          let hit_face = result.x();
          let hit = hit_face.not_equals(val(0.));
          let local_ray_range = local_ray_range.clone();
          if_by(hit, move || {
            let world_distance = result.y() / distance_scaling;

            let hit_kind = val(HIT_KIND_FRONT_FACING_TRIANGLE).make_local_var();
            if_by(hit_face.less_than(val(0.)), || {
              hit_kind.store(val(HIT_KIND_BACK_FACING_TRIANGLE));
            });

            // load tlas, write to hit ctx
            if_by(
              hit_ctx_curr
                .instance_id
                .load()
                .not_equals(ray_blas.tlas_idx),
              || {
                let ptr = tlas_data.index(ray_blas.tlas_idx);
                let instance_shader_binding_table_record_offset = TopLevelAccelerationStructureSourceDeviceInstance::readonly_storage_node_instance_shader_binding_table_record_offset_field_ptr(ptr).load();
                let instance_custom_index = TopLevelAccelerationStructureSourceDeviceInstance::readonly_storage_node_instance_custom_index_field_ptr(ptr).load();
                hit_ctx_curr.instance_id.store(ray_blas.tlas_idx);
                hit_ctx_curr
                  .instance_sbt_offset
                  .store(instance_shader_binding_table_record_offset);
                hit_ctx_curr
                  .instance_custom_id
                  .store(instance_custom_index);
              },
            );

            hit_ctx_curr.primitive_id.store(tri_idx - primitive_start);
            hit_ctx_curr.geometry_id.store(geometry_id);
            hit_ctx_curr.object_space_ray_origin.store(ray.origin);
            hit_ctx_curr.object_space_ray_direction.store(ray.direction);

            let hit_ctx = hit_ctx_curr.load(tlas_data);

            let attribute = BuiltInTriangleHitAttributeShaderAPIInstance {
              bary_coord: result.zw(),
            }
              .construct();

            // just to bundle data with no runtime cost. any_hit shader does not run.
            let any_hit_ctx = RayAnyHitCtx {
              launch_info,
              world_ray,
              hit_ctx,
              hit: HitInfo {
                hit_kind: hit_kind.load(),
                hit_distance: world_distance,
                hit_attribute: attribute,
              },
              payload: user_defined_payload,
            };

            if_by(is_opaque, || {
              // opaque -> commit
              closest_hit_var.test_and_store(&any_hit_ctx.hit, || {
                closest_hit_ctx_var.store(&any_hit_ctx.hit_ctx);
                local_ray_range.update_world_far(world_distance);
                if_by(flags.end_search_on_hit(), || end_search.store(true));
              });
            }).else_by(|| {
              // transparent trangle -> anyhit, then commit
              resolve_any_hit(
                |_| {
                  local_ray_range.update_world_far(world_distance);
                  if_by(flags.end_search_on_hit(), || end_search.store(true));
                },
                || end_search.store(true),
                any_hit,
                &any_hit_ctx,
                closest_hit_ctx_var,
                closest_hit_var,
              );
            });

            if_by(end_search.load(), || tri_loop.do_break());
          });
        });
        if_by(end_search.load(), || mesh_loop.do_break());
      });
    });

    // put intersect code here just in case
    // let intersect_ctx = RayIntersectCtx {
    //   launch_info,
    //   world_ray,
    //   hit_ctx,
    // };
    // // intersect will invoke any_hit and then update closest_hit.
    // intersect(
    //   &intersect_ctx,
    //   &NaiveIntersectReporter {
    //     launch_info,
    //     world_ray,
    //     hit_ctx,
    //     closest_hit_ctx_info: closest_hit_ctx_var,
    //     closest_hit_info: closest_hit_var,
    //     ray_range: local_ray_range.clone(),
    //     any_hit,
    //     on_end_search: Box::new(move || end_search.store(true)),
    //     user_defined_payload,
    //   },
    // );

    if_by(end_search.load(), || blas_loop.do_break());
  });
}

#[derive(Copy, Clone)]
struct HitCtxInfoVar {
  pub primitive_id: LocalVarNode<u32>,
  pub instance_id: LocalVarNode<u32>,
  pub instance_sbt_offset: LocalVarNode<u32>,
  pub instance_custom_id: LocalVarNode<u32>,
  pub geometry_id: LocalVarNode<u32>,
  pub object_space_ray_origin: LocalVarNode<Vec3<f32>>,
  pub object_space_ray_direction: LocalVarNode<Vec3<f32>>,
}
impl Default for HitCtxInfoVar {
  fn default() -> Self {
    Self {
      primitive_id: val(u32::MAX).make_local_var(),
      instance_id: val(u32::MAX).make_local_var(),
      instance_sbt_offset: val(u32::MAX).make_local_var(),
      instance_custom_id: val(u32::MAX).make_local_var(),
      geometry_id: val(u32::MAX).make_local_var(),
      object_space_ray_origin: val(vec3(0., 0., 0.)).make_local_var(),
      object_space_ray_direction: val(vec3(0., 0., 0.)).make_local_var(),
    }
  }
}
impl HitCtxInfoVar {
  fn load(
    &self,
    tlas_data: ReadOnlyStorageNode<[TopLevelAccelerationStructureSourceDeviceInstance]>,
  ) -> HitCtxInfo {
    if_by(self.instance_id.load().equals(val(u32::MAX)), || {
      self.instance_id.store(val(0));
    });
    let tlas_idx = self.instance_id.load();
    let tlas = tlas_data.index(tlas_idx);
    HitCtxInfo {
      primitive_id: self.primitive_id.load(),
      instance_id: tlas_idx,
      geometry_id: self.geometry_id.load(),
      instance_sbt_offset: self.instance_sbt_offset.load(),
      instance_custom_id: self.instance_custom_id.load(),
      object_to_world: TopLevelAccelerationStructureSourceDeviceInstance::readonly_storage_node_transform_field_ptr(tlas).load(),
      world_to_object: TopLevelAccelerationStructureSourceDeviceInstance::readonly_storage_node_transform_inv_field_ptr(
        tlas,
      ).load(),
      object_space_ray: ShaderRay {
        origin: self.object_space_ray_origin.load(),
        direction: self.object_space_ray_direction.load(),
      },
    }
  }
  fn store(&self, source: &HitCtxInfo) {
    self.primitive_id.store(source.primitive_id);
    self.instance_id.store(source.instance_id);
    self.instance_sbt_offset.store(source.instance_sbt_offset);
    self.instance_custom_id.store(source.instance_custom_id);
    self.geometry_id.store(source.geometry_id);
    self
      .object_space_ray_origin
      .store(source.object_space_ray.origin);
    self
      .object_space_ray_direction
      .store(source.object_space_ray.direction);
  }
}
#[derive(Clone)]
struct HitInfoVar {
  pub any_hit: LocalVarNode<bool>,
  pub hit_kind: LocalVarNode<u32>,
  pub hit_distance: LocalVarNode<f32>,
  pub hit_attribute: LocalVarNode<HitAttribute>,
}
impl Default for HitInfoVar {
  fn default() -> Self {
    Self {
      any_hit: val(false).make_local_var(),
      hit_kind: val(0).make_local_var(),
      hit_distance: val(f32::MAX).make_local_var(),
      hit_attribute: BuiltInTriangleHitAttributeShaderAPIInstance {
        bary_coord: val(vec2(0., 0.)),
      }
      .construct()
      .make_local_var(),
    }
  }
}
impl HitInfoVar {
  fn test_and_store(&self, source: &HitInfo, if_passed: impl FnOnce()) {
    if_by(
      source.hit_distance.less_than(self.hit_distance.load()),
      || {
        self.any_hit.store(val(true));
        self.hit_kind.store(source.hit_kind);
        self.hit_distance.store(source.hit_distance);
        self.hit_attribute.store(source.hit_attribute);
        if_passed();
      },
    );
  }
}
