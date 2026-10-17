use crate::*;

#[derive(Clone)]
pub struct SceneModelPickerBaseImplUtil {
  pub node_world: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub scene_model_node: ForeignKeyReadView<SceneModelRefNode>,
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Option<Box3<f64>>>,
  pub sm_local_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f32>>,
  pub selectable: ComponentReadView<SceneModelSelectable>,
}

impl SceneModelPickerBaseImplUtil {
  /// return the node of the model if the model passes the selectable and visible check
  pub fn pre_check(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ignore_pre_check: bool,
  ) -> Option<EntityHandle<SceneNodeEntity>> {
    let node = self.scene_model_node.get(idx)?;
    if ignore_pre_check {
      return Some(node);
    }
    if self.selectable.get(idx) != Some(&true) {
      return None;
    }
    if !self.node_net_visible.access(&node)? {
      return None;
    }
    Some(node)
  }

  /// resolve the world matrix and the world bounding of the model, if the override_world_mat is
  /// provided, the node matrix is ignored
  pub fn resolve_mat_and_world_aabb(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    node: EntityHandle<SceneNodeEntity>,
    override_world_mat: Option<&Mat4<f64>>,
  ) -> Option<(Mat4<f64>, Box3<f64>)> {
    if let Some(mat) = override_world_mat {
      let world_aabb = self
        .sm_local_bounding
        .access(&idx)?
        .into_f64()
        .apply_matrix_into(*mat);
      return Some((*mat, world_aabb));
    }

    let world = self.node_world.access(&node)?;
    let world_aabb = if let Some(world_aabb) = self.sm_world_bounding.access(&idx)? {
      world_aabb
    } else {
      self
        .sm_local_bounding
        .access(&idx)?
        .into_f64()
        .apply_matrix_into(world)
    };
    Some((world, world_aabb))
  }
}

/// The base scene model picker: pre check, bounding test, transform the query into local space
/// and dispatch it to the resolved local model picker.
pub struct SceneModelPickerBaseImpl<T> {
  pub util: SceneModelPickerBaseImplUtil,
  pub internal: T,
  /// the resolved local model pickers are allocated in it, it is reset after each model query
  pub scratch: ScratchAllocator,
}

struct ResolvedModel<'a> {
  local: ResolvedLocalModelPickerBox<'a>,
  mat: Mat4<f64>,
  /// enlarged by the local tolerance
  world_aabb: Box3<f64>,
  local_tolerance: f32,
}

fn log_pick_data_error(idx: EntityHandle<SceneModelEntity>, err: &LocalModelPickDataError) {
  log::error!("failed to pick scene model {idx:?}: {err}");
}

fn report_pick_data_error(
  idx: EntityHandle<SceneModelEntity>,
  r: Result<ControlFlow<()>, LocalModelPickDataError>,
) -> ControlFlow<()> {
  r.unwrap_or_else(|err| {
    log_pick_data_error(idx, &err);
    ControlFlow::Continue(())
  })
}

impl<T: LocalModelPickerResolver> SceneModelPickerBaseImpl<T> {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ignore_pre_check: bool,
    camera_ctx: &CameraQueryCtx,
    extra_screen_space_tolerance: f32,
    alloc: &'a dyn Allocator,
  ) -> Option<ResolvedModel<'a>> {
    let node = self.util.pre_check(idx, ignore_pre_check)?;
    let local = self
      .internal
      .resolve(idx, alloc)?
      .inspect_err(|err| log_pick_data_error(idx, err))
      .ok()?;
    let (mat, mut world_aabb) =
      self
        .util
        .resolve_mat_and_world_aabb(idx, node, override_world_mat)?;

    let max_scale = mat.max_scale();
    let center = world_aabb.center();
    let mut local_tolerance = local
      .tolerance()
      .map(|tolerance| camera_ctx.compute_local_tolerance(tolerance, max_scale, center))
      .unwrap_or(0.);

    if extra_screen_space_tolerance > 0. {
      let extra = IntersectTolerance::new(extra_screen_space_tolerance, ToleranceType::ScreenSpace);
      local_tolerance += camera_ctx.compute_local_tolerance(extra, max_scale, center);
    }

    if local_tolerance > 0. {
      world_aabb = world_aabb.enlarge(local_tolerance as f64 * max_scale);
    }

    Some(ResolvedModel {
      local,
      mat,
      world_aabb,
      local_tolerance,
    })
  }
}

impl<T: LocalModelPickerResolver> SceneModelPicker for SceneModelPickerBaseImpl<T> {
  fn ray_query(&self, req: SceneModelRayQuery, v: &mut dyn RayHitVisitor) -> ControlFlow<()> {
    self
      .scratch
      .scope(|alloc| self.ray_query_impl(req, v, alloc))
  }

  fn frustum_query(
    &self,
    req: SceneModelFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> ControlFlow<()> {
    self
      .scratch
      .scope(|alloc| self.frustum_query_impl(req, v, alloc))
  }
}

impl<T: LocalModelPickerResolver> SceneModelPickerBaseImpl<T> {
  fn ray_query_impl(
    &self,
    req: SceneModelRayQuery,
    v: &mut dyn RayHitVisitor,
    alloc: &dyn Allocator,
  ) -> ControlFlow<()> {
    let ctx = req.ctx;
    let Some(model) = self.resolve(
      req.idx,
      req.override_world_mat,
      req.ignore_pre_check,
      &ctx.camera_ctx,
      ctx.extra_screen_space_tolerance,
      alloc,
    ) else {
      return ControlFlow::Continue(());
    };

    match ray_aabb_entry_t(&ctx.world_ray, &model.world_aabb) {
      Some(t_enter) if t_enter <= v.t_max() => {}
      _ => return ControlFlow::Continue(()),
    }

    let local_ray = ctx
      .world_ray
      .apply_matrix_into(model.mat.inverse_or_identity())
      .into_f32();

    let local_req = LocalRayQuery {
      idx: req.idx,
      local_ray,
      local_tolerance: model.local_tolerance,
      extra_screen_space_tolerance: ctx.extra_screen_space_tolerance,
      world_mat: &model.mat,
      camera_ctx: &ctx.camera_ctx,
    };
    let mut adapter =
      LocalToWorldRayHitAdapter::new(v, req.idx, model.mat, ctx.world_ray, &local_ray);
    report_pick_data_error(
      req.idx,
      model.local.ray_query_local(local_req, &mut adapter),
    )
  }

  fn frustum_query_impl(
    &self,
    req: SceneModelFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
    alloc: &dyn Allocator,
  ) -> ControlFlow<()> {
    let frustum = req.frustum;
    let Some(model) = self.resolve(
      req.idx,
      req.override_world_mat,
      req.ignore_pre_check,
      &frustum.camera_ctx,
      frustum.extra_screen_space_tolerance,
      alloc,
    ) else {
      return ControlFlow::Continue(());
    };

    let coverage = if req.known_inside {
      IntersectResult::Inside
    } else {
      f_intersect_exact(
        frustum.world_helper.as_ref(),
        &frustum.world_frustum,
        &model.world_aabb,
      )
    };

    match coverage {
      // every standard visitor gives the correct result without any event
      IntersectResult::Outside => ControlFlow::Continue(()),
      IntersectResult::Inside => {
        let primitive_count = match model.local.primitive_count() {
          Ok(0) => return ControlFlow::Continue(()),
          Ok(count) => count,
          Err(err) => return report_pick_data_error(req.idx, Err(err)),
        };
        v.visit(FrustumPrimitiveEvent::whole(primitive_count, true))
      }
      IntersectResult::Intersect => {
        let local_frustum =
          transform_frustum_world_to_local(&frustum.world_frustum, &model.mat).into_f32();
        let helper = FrustumIntersectionTestHelper::new(&local_frustum);

        let local_req = LocalFrustumQuery {
          idx: req.idx,
          local_frustum: &local_frustum,
          helper: helper.as_ref(),
          policy: req.policy,
          extra_screen_space_tolerance: frustum.extra_screen_space_tolerance,
          world_mat: &model.mat,
          camera_ctx: &frustum.camera_ctx,
        };
        report_pick_data_error(req.idx, model.local.frustum_query_local(local_req, v))
      }
    }
  }
}
