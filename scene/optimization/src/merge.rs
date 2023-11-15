use std::{
  hash::Hasher,
  task::{Context, Poll},
};

use fast_hash_collection::*;
use incremental::IncrementalBase;

use crate::*;

pub struct SceneIncrementalMergeSystem {
  source_scene: Scene,
  optimized_scene: Scene,

  merge_relation: Box<dyn DynamicReactiveOneToManyRelationship<MergeKey, AllocIdx<SceneModelImpl>>>,
  // use to update mesh's vertex, the visibility is expressed by all zero matrix value
  applied_matrix_table: Box<dyn DynamicReactiveCollection<AllocIdx<SceneModelImpl>, Mat4<f32>>>,
  // all merged models
  merged_model: FastHashMap<MergeKey, ModelMerger>,
}

#[derive(Default)]
struct ModelMerger {
  //
}
impl ModelMerger {
  fn add_source(&mut self, source: AllocIdx<SceneModelImpl>) {}
  fn remove_source(&mut self, source: AllocIdx<SceneModelImpl>) {}
  fn notify_source_applied_matrix(&mut self, source: AllocIdx<SceneModelImpl>, mat: Mat4<f32>) {}

  /// return if has any active proxy exist after removal
  fn do_updates(
    &mut self,
    target_scene: &Scene,
    reverse_access: &dyn Fn(&mut dyn FnMut(AllocIdx<SceneModelImpl>)),
  ) -> bool {
    true
  }
}

impl SceneIncrementalMergeSystem {
  pub fn poll_update_merge(&mut self, cx: &mut Context) {
    let mut changed_key = FastHashSet::default();

    if let Poll::Ready(Some(changes)) = self.merge_relation.poll_changes_dyn(cx) {
      for change in changes {
        match change {
          CollectionDelta::Delta(source_idx, new_key, old_key) => {
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
          CollectionDelta::Remove(source_idx, key) => {
            self
              .merged_model
              .get_mut(&key)
              .unwrap()
              .remove_source(source_idx);
            changed_key.insert(key);
          }
        }
      }
    }

    let accessor = self.merge_relation.access_boxed();
    if let Poll::Ready(Some(changes)) = self.applied_matrix_table.poll_changes_dyn(cx) {
      for change in changes {
        if let CollectionDelta::Delta(source_idx, new_mat, _) = change {
          let merge_key = accessor(&source_idx).unwrap();
          self
            .merged_model
            .get_mut(&merge_key)
            .unwrap()
            .notify_source_applied_matrix(source_idx, new_mat)
        }
      }
    }

    let accessor = self.merge_relation.access_multi_boxed();
    for key in &changed_key {
      let merged = self.merged_model.get_mut(key).unwrap();
      let should_remove = merged.do_updates(&self.optimized_scene, &|f| {
        accessor(key, f);
      });
      if should_remove {
        self.merged_model.remove(key);
      }
    }
  }
}

pub type MaterialGUID = u64;
pub type MaterialContentID = u64;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum MergeKey {
  // not std model
  UnableToMerge(u64),
  Standard(StandardMergeKey),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeshMergeType {
  Attribute(u64),
  Foreign(u64),
}

pub fn build_merge_relation(
  scene_id: u64,
  source_scene_node_mat: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
  sm_node_relation: impl ReactiveOneToManyRelationship<NodeIdentity, AllocIdx<SceneModelImpl>>,
  std_sm_relation: impl ReactiveOneToManyRelationship<AllocIdx<StandardModel>, AllocIdx<SceneModelImpl>>
    + Clone,
) -> impl ReactiveCollection<AllocIdx<SceneModelImpl>, MergeKey> {
  // we only care about the given scene's scene models, to achieve this, simply check the scene id
  // on node change, the node's scene id itself is immutable.
  let node_checker = move |node: &SceneNode| {
    if node.scene_and_node_id().0 == scene_id {
      Some(Some(()))
    } else {
      Some(None)
    }
  };

  let referenced_sm = storage_of::<SceneModelImpl>()
    .listen_to_reactive_collection(move |change| match change {
      incremental::MaybeDeltaRef::Delta(delta) => match delta {
        SceneModelImplDelta::node(node) => node_checker(node),
        _ => None,
      },
      incremental::MaybeDeltaRef::All(sm) => node_checker(&sm.node),
    })
    .collective_filter_map(|v| v);

  let referenced_sm = referenced_sm.into_forker();

  let referenced_std_md = referenced_sm
    .clone()
    .many_to_one_reduce_key(std_sm_relation.clone());
  let referenced_std_md = referenced_std_md.into_forker();
  let mat_content_hash = sm_material_content_hash(&referenced_std_md);
  let mat_content_hash = mat_content_hash.one_to_many_fanout(std_sm_relation.clone());

  let std_mesh_key = std_mesh_key(&referenced_std_md);
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

  std_key
    .collective_union(referenced_sm)
    .collective_map(|(keyed, all)| match (keyed, all) {
      (Some(key), Some(all)) => MergeKey::Standard(key),
      (None, Some(all)) => MergeKey::UnableToMerge(todo!()),
      _ => unreachable!(),
    })
}

pub type SceneModelGUID = u64;
use std::hash::Hash;

fn sm_material_content_hash(
  std_scope: &(impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone),
  // todo, foreign material type support
  // foreign_materials_content_hash: impl FnOnce(
  //   Box<dyn DynamicReactiveCollection<AllocIdx<StandardModel>, ()>>,
  // ) -> Box<
  //   dyn DynamicReactiveCollection<AllocIdx<StandardModel>, MaterialContentID>,
  // >,
) -> impl ReactiveCollection<AllocIdx<StandardModel>, MaterialContentID> {
  // let foreign_material_hash = foreign_materials_content_hash(todo!());

  let flat = material_hash_impl::<FlatMaterial>(std_scope);
  let pbr_mr = material_hash_impl::<PhysicalMetallicRoughnessMaterial>(std_scope);
  let pbr_sg = material_hash_impl::<PhysicalSpecularGlossinessMaterial>(std_scope);

  // todo, impl another efficient multi select.
  flat.collective_select(pbr_mr).collective_select(pbr_sg)
  // .collective_select(foreign_material_hash)
}

fn material_hash_impl<M: IncrementalBase + Hash>(
  std_scope: &(impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone),
) -> impl ReactiveCollection<AllocIdx<StandardModel>, MaterialContentID> {
  // todo, create from global registry
  let relations: OneManyRelationForker<AllocIdx<M>, AllocIdx<StandardModel>> = todo!();

  let referenced_mat = std_scope.clone().many_to_one_reduce_key(relations.clone());
  //
  let material_hash = storage_of::<M>()
    .listen_to_reactive_collection(|_| Some(()))
    .filter_by_keyset(referenced_mat)
    .collective_execute_map_by(|| {
      let rehash = storage_of::<M>().create_key_mapper(|mat| {
        let mut hasher = FastHasher::default();
        mat.hash(&mut hasher);
        hasher.finish()
      });
      move |k, _| rehash(*k)
    });

  material_hash.one_to_many_fanout(relations)
}

// todo, foreign mesh support
fn std_mesh_key(
  std_scope: &(impl ReactiveCollection<AllocIdx<StandardModel>, ()> + Clone),
) -> impl ReactiveCollection<AllocIdx<StandardModel>, MeshMergeType> {
  // todo, create from global registry
  let relations: OneManyRelationForker<AllocIdx<AttributesMesh>, AllocIdx<StandardModel>> = todo!();

  let referenced_attribute_mesh = std_scope.clone().many_to_one_reduce_key(relations.clone());

  let attribute_key = storage_of::<AttributesMesh>()
    .listen_to_reactive_collection(|_| Some(()))
    .filter_by_keyset(referenced_attribute_mesh)
    .collective_execute_map_by(|| {
      let layout_key = storage_of::<AttributesMesh>().create_key_mapper(|mesh| {
        // todo, attribute layout key
        0
      });
      move |k, _| layout_key(*k)
    });
  attribute_key
    .one_to_many_fanout(relations)
    .collective_union(std_scope.clone())
    .collective_map(|(keyed, all)| match (keyed, all) {
      (Some(key), Some(all)) => MeshMergeType::Attribute(key),
      (None, Some(all)) => MeshMergeType::Foreign(todo!()),
      _ => unreachable!(),
    })
}
