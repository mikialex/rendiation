use std::any::Any;

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

  fn remove_all_models(&mut self, target_scene: &Scene) {
    for m in self.merged_model.drain(..) {
      target_scene.remove_model(m);
    }
  }
}

pub enum MergeUpdating {
  MergeTargetRemoved,
  SyncSingleModel {
    source: AllocIdx<StandardModel>,
    world_mat: Mat4<f32>,
  },
  DoUpdates(Box<dyn Any + Send + Sync>),
}
impl SceneModelMergeOptimization {
  pub(crate) fn commit_all_updates(&self, updates: Vec<(MergeKey, MergeUpdating)>) {
    updates.into_iter().for_each(|(key, update)| match update {
      MergeUpdating::MergeTargetRemoved => {
        self.merged_model.remove(&key);
      }
      MergeUpdating::SyncSingleModel { source, world_mat } => {
        let mut merge_proxy = self.merged_model.get_mut(&key).unwrap();
        merge_proxy.remove_all_models(&self.target_scene);

        let node = self.target_scene.create_root_child();
        node.set_local_matrix(world_mat);

        let model = storage_of::<StandardModel>().clone_at_idx(source).unwrap();
        let model = ModelEnum::Standard(model);
        let model = SceneModelImpl::new(model, node);

        merge_proxy
          .merged_model
          .push(self.target_scene.insert_model(model.into_ptr()));
      }
      MergeUpdating::DoUpdates(transaction) => {
        let mut merge_proxy = self.merged_model.get_mut(&key).unwrap();
        merge_proxy.remove_all_models(&self.target_scene);
        self.merge_methods.get_merge_impl_by_key(&key).commit_dyn(
          transaction,
          &mut merge_proxy,
          &self.target_scene,
        )
      }
    })
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

  pub fn do_updates(
    &mut self,
    key: &MergeKey,
    reg: &MergeImplRegistry,
    reverse_access: &dyn Fn(&mut dyn FnMut(AllocIdx<SceneModelImpl>)),
    mat_access: &dyn ReactiveCollection<AllocIdx<SceneModelImpl>, Mat4<f32>>,
  ) -> MergeUpdating {
    // only matrix/vis change, go fast path
    if !self.source_changed && self.mat_changed {
      // do only matrix update
      // do early return
    }
    self.reset_state();

    let mut source = Vec::new();
    reverse_access(&mut |source_idx| {
      source.push(source_idx);
    });

    if source.is_empty() {
      self.reset_state();
      return MergeUpdating::MergeTargetRemoved;
    }

    let mat_access = mat_access.make_accessor();

    // todo reuse code
    let sm_storage = storage_of::<SceneModelImpl>();
    let sm_storage_data = sm_storage.inner.data.read();
    let std_models = source
      .iter()
      .map(|sm| {
        let sm_data = sm_storage_data.get(sm.index);
        match &sm_data.data.model {
          ModelEnum::Standard(m) => m.alloc_index().into(),
          ModelEnum::Foreign(_) => unreachable!(),
        }
      })
      .collect::<Vec<_>>();

    if source.len() == 1 {
      // for single output, we directly sync mat instead of apply the mat on mesh
      MergeUpdating::SyncSingleModel {
        source: std_models[0],
        world_mat: mat_access(&source[0]).unwrap(),
      }
    } else {
      let transforms = source
        .iter()
        .map(|idx| mat_access(idx).unwrap())
        .collect::<Vec<_>>();
      let ctx = MeshMergeCtx {
        models: &std_models,
        transforms: &transforms,
        key,
      };
      let merge_transaction = reg.get_merge_impl_by_key(key).prepare_dyn(&ctx);

      MergeUpdating::DoUpdates(merge_transaction)
    }
  }
}

pub struct MeshMergeCtx<'a> {
  pub models: &'a [AllocIdx<StandardModel>],
  pub transforms: &'a [Mat4<f32>],
  pub key: &'a MergeKey,
}

// impl MeshMergeSource for
pub struct MergeImplRegistry {
  implementations: Vec<Box<dyn MergeImplementationBoxed>>,
}

pub trait MergeImplementation: Send + Sync {
  type Transaction: Any + Send + Sync;
  fn prepare(&self, ctx: &MeshMergeCtx) -> Self::Transaction;
  fn commit(&self, trans: Self::Transaction, proxy: &mut ModelMergeProxy, target: &Scene);
}

pub trait MergeImplementationBoxed: Send + Sync {
  fn prepare_dyn(&self, ctx: &MeshMergeCtx) -> Box<dyn Any + Send + Sync>;
  fn commit_dyn(
    &self,
    trans: Box<dyn Any + Send + Sync>,
    proxy: &mut ModelMergeProxy,
    target: &Scene,
  );
}
impl<T: MergeImplementation> MergeImplementationBoxed for T {
  fn prepare_dyn(&self, ctx: &MeshMergeCtx) -> Box<dyn Any + Send + Sync> {
    Box::new(self.prepare(ctx))
  }
  fn commit_dyn(
    &self,
    trans: Box<dyn Any + Send + Sync>,
    proxy: &mut ModelMergeProxy,
    target: &Scene,
  ) {
    self.commit(*trans.downcast::<T::Transaction>().unwrap(), proxy, target)
  }
}

pub const ATTRIBUTE_MERGE: usize = 0;
impl Default for MergeImplRegistry {
  fn default() -> Self {
    let mut s = Self {
      implementations: Default::default(),
    };
    s.register(AttributesMeshMergeImpl);

    s
  }
}

impl MergeImplRegistry {
  pub fn register(&mut self, implementation: impl MergeImplementation + 'static) -> usize {
    self.implementations.push(Box::new(implementation));
    self.implementations.len() - 1
  }

  pub fn get_merge_impl_by_key(&self, key: &MergeKey) -> &dyn MergeImplementationBoxed {
    match key {
      MergeKey::Standard(std) => match std.mesh_layout_type {
        MeshMergeType::Mergeable(merge_type, _) => self
          .get_merge_impl(merge_type)
          .expect("merge method has not registered"),
        _ => unreachable!("merge key is invalid when get merge impl"),
      },
      _ => unreachable!("merge key is invalid when get merge impl"),
    }
  }

  pub fn get_merge_impl(&self, id: usize) -> Option<&dyn MergeImplementationBoxed> {
    self.implementations.get(id).map(|v| v.as_ref())
  }
}

struct AttributesMeshMergeImpl;

struct AttributeMergeTransaction {
  mesh_data: AttributeMeshData,
  // this is used to sync material
  first_source_model: AllocIdx<StandardModel>,
  back_face: bool,
}

impl MergeImplementation for AttributesMeshMergeImpl {
  type Transaction = Vec<AttributeMergeTransaction>;

  fn prepare(&self, ctx: &MeshMergeCtx) -> Self::Transaction {
    let std_storage = storage_of::<StandardModel>();
    let std_storage_data = std_storage.inner.data.read();

    let mesh_storage = storage_of::<AttributesMesh>();
    let mesh_storage_data = mesh_storage.inner.data.read();

    let back_face = match ctx.key {
      MergeKey::UnableToMergeNoneStandard(_) => false,
      MergeKey::Standard(key) => !key.world_mat_is_front_side,
    };

    // refs
    let sources = ctx
      .models
      .iter()
      .map(|m| {
        let m = &std_storage_data.get(m.index).data;
        match &m.mesh {
          MeshEnum::AttributesMesh(mesh) => &mesh_storage_data.get(mesh.alloc_index()).data,
          _ => unreachable!(),
        }
      })
      .collect::<Vec<_>>();

    // we should compute it first to avoid recompute for every vertex
    let position_mats = ctx
      .transforms
      .iter()
      .map(|mat| {
        if back_face {
          Mat4::scale((-1.0, 1., 1.)) * *mat
        } else {
          *mat
        }
      })
      .collect::<Vec<_>>();

    let normal_mats = ctx
      .transforms
      .iter()
      .map(|mat| mat.to_normal_matrix())
      .collect::<Vec<_>>();

    // do merge
    let meshes = merge_attributes_meshes(
      u32::MAX,
      &sources,
      |idx, position| position_mats[idx] * *position,
      |idx, normal| normal_mats[idx] * *normal,
    )
    .unwrap();

    // wrap results
    meshes
      .into_iter()
      .map(|m| AttributeMergeTransaction {
        mesh_data: m,
        first_source_model: ctx.models[0],
        back_face,
      })
      .collect()
  }

  fn commit(&self, trans: Self::Transaction, proxy: &mut ModelMergeProxy, target_scene: &Scene) {
    trans.into_iter().for_each(|tran| {
      let model = storage_of::<StandardModel>()
        .clone_at_idx(tran.first_source_model)
        .unwrap();
      let first_material = model.read().material.clone();
      let mesh = tran.mesh_data.build();
      let mesh = MeshEnum::AttributesMesh(mesh.into_ptr());
      let model = StandardModel::new(first_material, mesh).into_ptr();
      let model = ModelEnum::Standard(model);
      let node = target_scene.create_root_child();
      if tran.back_face {
        node.set_local_matrix(Mat4::scale((-1.0, 1.0, 1.0)));
      }
      let model = SceneModelImpl::new(model, node).into_ptr();
      let model = target_scene.insert_model(model);
      proxy.merged_model.push(model)
    });
  }
}
