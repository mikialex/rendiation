use smallvec::SmallVec;

use crate::*;

#[derive(Default)]
pub struct ModelMergeProxy {
  merged_model: SmallVec<[SceneModelHandle; 3]>,
  source_changed: bool,
  mat_changed: bool,
}

impl ModelMergeProxy {
  fn reset_state(&mut self) {
    self.source_changed = false;
    self.mat_changed = false;
  }
}

impl ModelMergeProxy {
  // in future we could exploit this fine grained change to make update more efficient.
  pub fn add_source(&mut self, _: AllocIdx<SceneModelImpl>) {
    self.source_changed = true;
  }
  pub fn remove_source(&mut self, _: AllocIdx<SceneModelImpl>) {
    self.source_changed = true;
  }
  pub fn notify_source_applied_matrix(&mut self, _: AllocIdx<SceneModelImpl>, _: Mat4<f32>) {
    self.mat_changed = true;
  }

  /// return if has any active proxy exist after removal
  pub fn do_updates(
    &mut self,
    target_scene: &Scene,
    key: &MergeKey,
    reg: &MergeImplRegistry,
    reverse_access: &dyn Fn(&mut dyn FnMut(AllocIdx<SceneModelImpl>)),
    mat_access: &dyn DynamicReactiveCollection<AllocIdx<SceneModelImpl>, Mat4<f32>>,
  ) -> bool {
    // only matrix/vis change, go fast path
    if !self.source_changed && self.mat_changed {
      // do only matrix update
      self.reset_state()
      // do early return
    }

    // remove and drop all previous models
    for m in self.merged_model.drain(..) {
      target_scene.remove_model(m);
    }

    let scene_models = storage_of::<SceneModelImpl>();
    let scene_models_data = scene_models.inner.data.read_recursive();
    let mut source = Vec::new();
    reverse_access(&mut |source_idx| {
      let m = scene_models_data.get(source_idx.index).data.model.clone();
      source.push((m, source_idx));
    });
    drop(scene_models_data);
    drop(scene_models);

    if source.is_empty() {
      self.reset_state();
      return false;
    }

    let mut results = Vec::new();
    let mat_access = mat_access.access_boxed();
    if source.len() == 1 {
      // for single output, we directly sync mat instead of apply the mat on mesh
      let node = target_scene.create_root_child();
      node.set_local_matrix(mat_access(&source[0].1).unwrap());
      results.push(SceneModelImpl::new(source[0].0.clone(), node));
    } else {
      let meshes = source
        .iter()
        .map(|(m, _)| match m {
          ModelEnum::Standard(m) => m.read().mesh.clone(),
          ModelEnum::Foreign(_) => unreachable!(),
        })
        .collect::<Vec<_>>();
      let transforms = source
        .iter()
        .map(|(_, idx)| mat_access(idx).unwrap())
        .collect::<Vec<_>>();
      let ctx = MeshMergeCtx {
        meshes: &meshes,
        transforms: &transforms,
      };
      let merge_method = match key {
        MergeKey::Standard(std) => match std.mesh_layout_type {
          MeshMergeType::Mergeable(merge_type, _) => reg.get_merge_impl(merge_type).unwrap(),
          _ => unreachable!(),
        },
        _ => unreachable!(),
      };

      let merged_mesh = merge_method(&ctx);
      let first_material = match &source[0].0 {
        ModelEnum::Standard(model) => model.read().material.clone(),
        _ => unreachable!(),
      };
      merged_mesh.iter().for_each(|mesh| {
        let model = StandardModel::new(first_material.clone(), mesh.clone()).into_ptr();
        let model = ModelEnum::Standard(model);
        let node = target_scene.create_root_child();

        let first_mat = mat_access(&source[0].1).unwrap();
        if first_mat.to_mat3().det() < 0. {
          node.set_local_matrix(Mat4::scale((-1.0, 1.0, 1.0)));
        }

        results.push(SceneModelImpl::new(model, node));
      });
    }

    self.merged_model = results
      .into_iter()
      .map(|model| target_scene.insert_model(model.into_ptr()))
      .collect();

    self.reset_state();
    true
  }
}

pub struct MeshMergeCtx<'a> {
  pub meshes: &'a [MeshEnum],
  pub transforms: &'a [Mat4<f32>],
}

// impl MeshMergeSource for
pub struct MergeImplRegistry {
  implementation: Vec<Box<dyn Fn(&MeshMergeCtx) -> Vec<MeshEnum> + Send + Sync>>,
}

impl Default for MergeImplRegistry {
  fn default() -> Self {
    let mut s = Self {
      implementation: Default::default(),
    };
    s.register(merge_attribute_mesh);

    s
  }
}

impl MergeImplRegistry {
  pub fn register(
    &mut self,
    f: impl Fn(&MeshMergeCtx) -> Vec<MeshEnum> + Send + Sync + 'static,
  ) -> usize {
    self.implementation.push(Box::new(f));
    self.implementation.len() - 1
  }

  pub fn get_merge_impl(&self, id: usize) -> Option<&dyn Fn(&MeshMergeCtx) -> Vec<MeshEnum>> {
    self
      .implementation
      .get(id)
      .map(|f| f as &dyn Fn(&MeshMergeCtx) -> Vec<MeshEnum>)
  }
}

fn merge_attribute_mesh(ctx: &MeshMergeCtx) -> Vec<MeshEnum> {
  // locks
  let sources = ctx
    .meshes
    .iter()
    .map(|m| match m {
      MeshEnum::AttributesMesh(m) => m.read(),
      _ => unreachable!(),
    })
    .collect::<Vec<_>>();

  // refs
  let s = sources
    .iter()
    .map(|m| m as &AttributesMesh)
    .collect::<Vec<_>>();

  // do merge
  let meshes = merge_attributes_meshes(
    u32::MAX,
    &s,
    |idx, position| position.apply_matrix_into(ctx.transforms[idx]),
    |idx, normal| normal.apply_matrix_into(ctx.transforms[idx].to_normal_matrix().into()),
  )
  .unwrap();

  // wrap results
  meshes
    .into_iter()
    .map(|m| MeshEnum::AttributesMesh(m.into_ptr()))
    .collect()
}

pub const ATTRIBUTE_MERGE: usize = 0;
