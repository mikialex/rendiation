use crate::*;

mod merge_impl;
use merge_impl::*;

type FastDashMap<K, V> = dashmap::DashMap<K, V, FastHasherBuilder>;
type FastDashSet<K> = dashmap::DashSet<K, FastHasherBuilder>;
use rayon::prelude::*;

pub struct SceneMergeSystem {
  models: SceneModelMergeOptimization,
  cameras: SceneCameraRebuilder,
  lights: SceneLightsRebuilder,
  pub target_scene: (Scene, NodeIncrementalDeriveCollections),
}

impl SceneMergeSystem {
  pub fn new(
    source_scene: &Scene,
    source_scene_derives: &NodeIncrementalDeriveCollections,
    foreign_merge_support: Box<dyn FnOnce(&mut MergeImplRegistry) -> Box<ForeignMergeKeySupport>>,
  ) -> (Self, Scene) {
    let (target_scene, _) = SceneImpl::new();
    let scene_derived =
      NodeIncrementalDeriveCollections::new(&target_scene.read().core.read().nodes);

    let source_id = source_scene.guid();

    let models = SceneModelMergeOptimization::new(
      source_id,
      source_scene_derives,
      &target_scene,
      foreign_merge_support,
    );

    let cameras = SceneCameraRebuilder::new(source_id, source_scene_derives, &target_scene);
    let lights = SceneLightsRebuilder::new(source_id, source_scene_derives, &target_scene);

    (
      Self {
        models,
        cameras,
        lights,
        target_scene: (target_scene.clone(), scene_derived),
      },
      target_scene,
    )
  }

  pub fn poll_updates(&mut self, cx: &mut Context) {
    self.models.poll_update_merge(cx);
    self.cameras.poll_updates(cx);
    self.lights.poll_updates(cx);
  }
}

pub struct SceneModelMergeOptimization {
  target_scene: Scene,

  merge_relation: Box<dyn ReactiveOneToManyRelationship<MergeKey, AllocIdx<SceneModelImpl>>>,
  // use to update mesh's vertex, the visibility is expressed by all zero matrix value
  applied_matrix_table: Box<dyn ReactiveCollection<AllocIdx<SceneModelImpl>, Mat4<f32>>>,
  // all merged models
  merged_model: FastDashMap<MergeKey, ModelMergeProxy>,
  merge_methods: MergeImplRegistry,
}

impl SceneModelMergeOptimization {
  pub fn new(
    source_scene_id: u64,
    source_scene_derives: &NodeIncrementalDeriveCollections,
    target_scene: &Scene,
    foreign_merge_support: Box<dyn FnOnce(&mut MergeImplRegistry) -> Box<ForeignMergeKeySupport>>,
  ) -> Self {
    let target_scene = target_scene.clone();
    let source_scene_node_mat = source_scene_derives.world_mat.clone();
    let source_scene_node_net_vis = source_scene_derives.net_visible.clone();

    let mut merge_methods = MergeImplRegistry::default();
    let foreign_key_support = foreign_merge_support(&mut merge_methods);

    let merge_relation = build_merge_relation(
      source_scene_id,
      source_scene_node_mat.clone(),
      foreign_key_support,
    );

    let applied_matrix_table = source_scene_node_mat
      .collective_zip(source_scene_node_net_vis)
      .collective_map(|(mat, vis)| {
        if !vis {
          Mat4::zero()
        } else {
          // check if is front side
          if mat.to_mat3().det().is_sign_positive() {
            mat
          } else {
            Mat4::scale((-1.0, 1.0, 1.0)) * mat
          }
        }
      })
      .one_to_many_fanout(scene_model_ref_node_many_one_relation());

    Self {
      target_scene,
      merge_relation: Box::new(merge_relation.into_one_to_many_by_hash()),
      applied_matrix_table: Box::new(applied_matrix_table),
      merged_model: Default::default(),
      merge_methods,
    }
  }
}

impl SceneModelMergeOptimization {
  pub fn poll_update_merge(&mut self, cx: &mut Context) {
    let updates = self.poll_prepare_merge(cx);
    self.commit_all_updates(updates);
  }

  pub(crate) fn poll_prepare_merge(&mut self, cx: &mut Context) -> Vec<(MergeKey, MergeUpdating)> {
    let changed_key = FastDashSet::default();
    if let Poll::Ready(changes) = self.merge_relation.poll_changes(cx) {
      changes
        .iter_key_value()
        .for_each(|(source_idx, change)| match change {
          ValueChange::Delta(new_key, old_key) => {
            self
              .merged_model
              .entry(new_key)
              .or_default()
              .add_source(source_idx);
            changed_key.insert(new_key);
            if let Some(old_key) = old_key {
              self
                .merged_model
                .get_mut(&old_key)
                .unwrap()
                .remove_source(source_idx);

              changed_key.insert(old_key);
            }
          }
          ValueChange::Remove(key) => {
            self
              .merged_model
              .get_mut(&key)
              .unwrap()
              .remove_source(source_idx);
            changed_key.insert(key);
          }
        })
    }

    let accessor = self.merge_relation.make_accessor();
    if let Poll::Ready(changes) = self.applied_matrix_table.poll_changes(cx) {
      changes.iter_key_value().for_each(|(source_idx, change)| {
        if let ValueChange::Delta(new_mat, _) = change {
          let merge_key = accessor(&source_idx).unwrap();
          self
            .merged_model
            .get_mut(&merge_key)
            .unwrap()
            .notify_source_applied_matrix(source_idx, new_mat)
        }
      })
    }

    let accessor = self.merge_relation.make_multi_accessor();
    changed_key
      .into_par_iter()
      .map(|key| {
        let mut merged = self.merged_model.get_mut(&key).unwrap();
        let update = merged.do_updates(
          &key,
          &self.merge_methods,
          &|f| {
            accessor(&key, f);
          },
          &self.applied_matrix_table,
        );
        (key, update)
      })
      .collect()
  }
}

pub type MaterialGUID = u64;
pub type MaterialContentID = u64;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MergeKey {
  // not std model
  UnableToMergeNoneStandard(u64),
  Standard(StandardMergeKey),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StandardMergeKey {
  /// only same material could be merged together, here we not using material guid, instead, using
  /// another id to identify the same material content even if the material reference is
  /// different.
  pub material_content_id: MaterialContentID,
  pub mesh_layout_type: MeshMergeType,
  /// note, currently, we have or may have the auto reverse face in pipeline selection, if this
  /// automation exists, we have to add extra key here because we can not rely on the user set
  /// different material state
  pub world_mat_is_front_side: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MeshMergeType {
  // (merge_typeid, source_id)
  Mergeable(usize, u64),
  // should using unique id
  UnableToMerge(u64),
}

pub type ForeignMergeKeySupport = dyn FnOnce(
  RxCForker<AllocIdx<StandardModel>, ()>,
) -> (
  Box<dyn ReactiveCollection<AllocIdx<StandardModel>, MaterialContentID>>,
  Box<dyn ReactiveCollection<AllocIdx<StandardModel>, MeshMergeType>>,
);

pub fn build_merge_relation(
  scene_id: u64,
  source_scene_node_mat: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
  foreign: Box<ForeignMergeKeySupport>,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, MergeKey> {
  let node_checker = create_scene_node_checker(scene_id);
  let std_sm_relation = scene_model_ref_std_model_many_one_relation();
  let sm_node_relation = scene_model_ref_node_many_one_relation();

  let referenced_sm =
    storage_of::<SceneModelImpl>().listen_to_reactive_collection(move |change| match change {
      incremental::MaybeDeltaRef::Delta(delta) => match delta {
        SceneModelImplDelta::node(node) => ChangeReaction::Care(node_checker(node)),
        _ => ChangeReaction::NotCare,
      },
      incremental::MaybeDeltaRef::All(sm) => ChangeReaction::Care(node_checker(&sm.node)),
    });

  let referenced_sm = referenced_sm.into_forker();

  let referenced_sm_c = referenced_sm.clone();

  let referenced_std_md = referenced_sm
    .clone()
    .many_to_one_reduce_key(std_sm_relation.clone());

  let referenced_std_md = Box::new(referenced_std_md) as Box<dyn ReactiveCollection<_, _>>;
  let referenced_std_md = referenced_std_md.into_forker();

  let (foreign_mat, foreign_mesh) = foreign(referenced_std_md.clone());

  let mat_content_hash = sm_material_content_hash(&referenced_std_md, foreign_mat);
  let mat_content_hash = mat_content_hash.one_to_many_fanout(std_sm_relation.clone());

  let std_mesh_key = std_mesh_key(&referenced_std_md, foreign_mesh);
  let sm_mesh_key = std_mesh_key.one_to_many_fanout(std_sm_relation);

  let sm_front_face = source_scene_node_mat
    .one_to_many_fanout(sm_node_relation)
    .collective_map(|mat| mat.det().is_sign_positive());

  // todo  impl another efficient multi intersect.
  // we can not guarantee their key scope is aligned due to the unprovided foreign impl, so use
  // intersect
  let std_key = mat_content_hash
    .collective_intersect(sm_mesh_key)
    .collective_intersect(sm_front_face)
    .collective_map(|((mat, mesh), face)| StandardMergeKey {
      material_content_id: mat,
      mesh_layout_type: mesh,
      world_mat_is_front_side: face,
    });

  let referenced_sm_guid = referenced_sm_c.collective_execute_map_by(|| {
    let guid_getter = storage_of::<SceneModelImpl>().create_key_mapper(|_, guid| guid);
    move |k, _| guid_getter(*k)
  });

  std_key.collective_union(referenced_sm_guid, |(keyed, all)| match (keyed, all) {
    (Some(key), Some(_)) => MergeKey::Standard(key).into(),
    (None, Some(guid)) => MergeKey::UnableToMergeNoneStandard(guid).into(),
    _ => None,
  })
}

pub type SceneModelGUID = u64;
use std::{hash::Hash, task::Poll};

fn sm_material_content_hash(
  std_scope: &(impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone),
  foreign: Box<dyn ReactiveCollection<AllocIdx<StandardModel>, MaterialContentID>>,
) -> impl ReactiveCollection<AllocIdx<StandardModel>, MaterialContentID> {
  // let foreign_material_hash = foreign_materials_content_hash(todo!());

  let flat = material_hash_impl::<FlatMaterial>(std_scope);
  let pbr_mr = material_hash_impl::<PhysicalMetallicRoughnessMaterial>(std_scope);
  let pbr_sg = material_hash_impl::<PhysicalSpecularGlossinessMaterial>(std_scope);

  // todo, impl another efficient multi select.
  flat
    .collective_select(pbr_mr)
    .collective_select(pbr_sg)
    .collective_select(foreign)
}

fn material_hash_impl<M: DowncastFromMaterialEnum + Hash>(
  std_scope: &(impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone),
) -> impl ReactiveCollection<AllocIdx<StandardModel>, MaterialContentID> {
  let relations = global_material_relations::<M>();
  let referenced_mat = std_scope.clone().many_to_one_reduce_key(relations.clone());

  let material_hash = storage_of::<M>()
    .listen_all_instance_changed_set()
    .filter_by_keyset(referenced_mat)
    .collective_execute_simple_map(|mat| {
      let mut hasher = FastHasher::default();
      mat.hash(&mut hasher);
      hasher.finish()
    });

  material_hash.one_to_many_fanout(relations)
}

fn std_mesh_key(
  std_scope: &(impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone),
  foreign: Box<dyn ReactiveCollection<AllocIdx<StandardModel>, MeshMergeType>>,
) -> impl ReactiveCollection<AllocIdx<StandardModel>, MeshMergeType> {
  let referenced_attribute_mesh = std_scope
    .clone()
    .many_to_one_reduce_key(std_model_ref_att_mesh_many_one_relation());

  let attribute_key = storage_of::<AttributesMesh>()
    .listen_all_instance_changed_set()
    .filter_by_keyset(referenced_attribute_mesh)
    .collective_execute_map_by(|| {
      let layout_key = storage_of::<AttributesMesh>().create_key_mapper(|mesh, _| {
        // todo, filter not valid attribute layout key
        compute_merge_key(&mesh);
        0
      });
      move |k, _| layout_key(*k)
    });

  let std_scope = std_scope.clone().collective_execute_map_by(|| {
    let guid_getter = storage_of::<StandardModel>().create_key_mapper(|_, guid| guid);
    move |k, _| guid_getter(*k)
  });

  attribute_key
    .one_to_many_fanout(std_model_ref_att_mesh_many_one_relation())
    .collective_union(std_scope, |(keyed, all)| match (keyed, all) {
      (Some(key), Some(_)) => MeshMergeType::Mergeable(ATTRIBUTE_MERGE, key).into(),
      (None, Some(guid)) => MeshMergeType::UnableToMerge(guid).into(),
      _ => None,
    })
    .collective_select(foreign)
}
