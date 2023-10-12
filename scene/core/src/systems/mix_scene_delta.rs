use futures::StreamExt;
use reactive::{RemoveToken, SignalStreamExt};
use tree::{AbstractParentAddressableTreeNode, CoreTree, TreeMutation};

use crate::*;

/// compare to scene inner delta, this mixed delta support multi scene content mixing
#[derive(Clone)]
#[allow(non_camel_case_types)]
pub enum MixSceneDelta {
  background(DeltaOf<Option<SceneBackGround>>),
  active_camera(DeltaOf<Option<SceneCamera>>),
  cameras(ContainerRefRetainContentDelta<SceneCamera>),
  lights(ContainerRefRetainContentDelta<SceneLight>),
  models(ContainerRefRetainContentDelta<SceneModel>),
  ext(DeltaOf<DynamicExtension>),
}

impl std::fmt::Debug for MixSceneDelta {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::background(_) => f.debug_tuple("background").finish(),
      Self::active_camera(_) => f.debug_tuple("active_camera").finish(),
      Self::cameras(_) => f.debug_tuple("cameras").finish(),
      Self::lights(_) => f.debug_tuple("lights").finish(),
      Self::models(_) => f.debug_tuple("models").finish(),
      Self::ext(_) => f.debug_tuple("ext").finish(),
    }
  }
}

pub fn map_scene_delta_to_mixed(
  input: impl Stream<Item = SceneInternalDelta> + Unpin,
) -> impl Stream<Item = MixSceneDelta> {
  let input = input.create_broad_caster();

  let cameras = input
    .fork_stream()
    .filter_map_sync(|delta| match delta {
      SceneInternalDelta::cameras(c) => Some(c),
      _ => None,
    })
    .map(IndependentItemContainerDelta::from)
    .transform_delta_to_ref_retained_by_hashing()
    .transform_ref_retained_to_ref_retained_content_by_hashing()
    .map(MixSceneDelta::cameras);

  let lights = input
    .fork_stream()
    .filter_map_sync(|delta| match delta {
      SceneInternalDelta::lights(c) => Some(c),
      _ => None,
    })
    .map(IndependentItemContainerDelta::from)
    .transform_delta_to_ref_retained_by_hashing()
    .transform_ref_retained_to_ref_retained_content_by_hashing()
    .map(MixSceneDelta::lights);

  let models = input
    .fork_stream()
    .filter_map_sync(|delta| match delta {
      SceneInternalDelta::models(c) => Some(c),
      _ => None,
    })
    .map(IndependentItemContainerDelta::from)
    .transform_delta_to_ref_retained_by_hashing()
    .transform_ref_retained_to_ref_retained_content_by_hashing()
    .map(MixSceneDelta::models);

  let others = input.fork_stream().filter_map_sync(|delta| match delta {
    SceneInternalDelta::background(b) => MixSceneDelta::background(b).into(),
    SceneInternalDelta::active_camera(c) => MixSceneDelta::active_camera(c).into(),
    SceneInternalDelta::ext(ext) => MixSceneDelta::ext(ext).into(),
    _ => None,
  });

  let output = futures::stream::select(cameras, lights);
  let output = futures::stream::select(output, models);
  futures::stream::select(output, others)
}

pub fn mix_scene_folding(
  input: impl Stream<Item = MixSceneDelta>,
) -> (
  impl Stream<Item = MixSceneDelta>,
  (Scene, SceneNodeDeriveSystem),
) {
  let (scene, derives) = SceneImpl::new();

  let s = scene.clone();

  let sc = scene.get_scene_core();
  let nodes = sc.read().nodes.clone();
  let rebuilder = SceneRebuilder::new(nodes);
  let rebuilder = Arc::new(RwLock::new(rebuilder));
  let mut model_handle_map: FastHashMap<u64, SceneModelHandle> = Default::default();
  let mut camera_handle_map: FastHashMap<u64, SceneCameraHandle> = Default::default();
  let mut light_handle_map: FastHashMap<u64, SceneLightHandle> = Default::default();

  let output = input.map(move |delta| {
    //
    match &delta {
      MixSceneDelta::background(bg) => {
        s.set_background(bg.clone().map(merge_maybe));
      }
      MixSceneDelta::active_camera(camera) => {
        let mapped_camera = camera.as_ref().map(merge_maybe_ref).map(|camera| {
          let mapped_camera = camera_handle_map.entry(camera.guid()).or_insert_with(|| {
            let new = transform_camera_node(camera, &rebuilder);
            s.insert_camera(new)
          });
          s.read()
            .core
            .read()
            .cameras
            .get(*mapped_camera)
            .unwrap()
            .clone()
        });

        s.set_active_camera(mapped_camera);
      }
      MixSceneDelta::cameras(camera) => match camera {
        ContainerRefRetainContentDelta::Remove(camera) => {
          let (_, remover) = make_add_remover(&rebuilder);
          remover(camera.read().node.clone());
          let handle = camera_handle_map.remove(&camera.guid()).unwrap();
          s.remove_camera(handle);
        }
        ContainerRefRetainContentDelta::Insert(camera) => {
          let new = transform_camera_node(camera, &rebuilder);
          let new_handle = s.insert_camera(new);
          camera_handle_map.insert(camera.guid(), new_handle);
        }
      },
      MixSceneDelta::lights(light) => match light {
        ContainerRefRetainContentDelta::Remove(light) => {
          let (_, remover) = make_add_remover(&rebuilder);
          remover(light.read().node.clone());
          let handle = light_handle_map.remove(&light.guid()).unwrap();
          s.remove_light(handle);
        }
        ContainerRefRetainContentDelta::Insert(light) => {
          let new = transform_light_node(light, &rebuilder);
          let new_handle = s.insert_light(new);
          light_handle_map.insert(light.guid(), new_handle);
        }
      },
      MixSceneDelta::models(model) => match model {
        ContainerRefRetainContentDelta::Remove(model) => {
          let (_, remover) = make_add_remover(&rebuilder);
          remover(model.read().node.clone());
          let handle = model_handle_map.remove(&model.guid()).unwrap();
          s.remove_model(handle)
        }
        ContainerRefRetainContentDelta::Insert(model) => {
          // todo, should we check the inserted model has been mapped?
          let new = transform_model_node(model, &rebuilder);
          let new_handle = s.insert_model(new);
          model_handle_map.insert(model.guid(), new_handle);
        }
      },
      MixSceneDelta::ext(ext) => {
        s.update_ext(ext.clone());
      }
    }

    delta
  });

  (output, (scene, derives))
}

fn make_add_remover(
  rebuilder: &ShareableRebuilder,
) -> (
  impl Fn(SceneNode) -> SceneNode + Send + Sync + 'static, // todo, make input param pass by ref
  impl Fn(SceneNode) + Send + Sync + 'static,
) {
  let rebuilder = rebuilder.clone();
  let rebuilder2 = rebuilder.clone();
  let adder = move |node| add_entity_used_node(&rebuilder, &node);
  let remover = move |node| remove_entity_used_node(&rebuilder2, &node);
  (adder, remover)
}

pub fn pass_changes_to<T: IncrementalBase>(
  source: &IncrementalSignalPtr<T>,
  other: &IncrementalSignalPtr<T>,
  mut extra_mapper: impl FnMut(T::Delta) -> T::Delta + Send + Sync + 'static,
) where
  T: ApplicableIncremental,
{
  let other_weak = other.downgrade();
  // here we not care the listener removal because we use weak
  source.on(move |delta| {
    if let Some(other) = other_weak.upgrade() {
      other.mutate(|mut m| m.modify(extra_mapper(delta.clone())));
      false
    } else {
      true
    }
  });
}

fn transform_camera_node(m: &SceneCamera, rebuilder: &ShareableRebuilder) -> SceneCamera {
  let (adder, remover) = make_add_remover(rebuilder);

  let camera = m.read();
  let r = SceneCameraImpl {
    bounds: camera.bounds,
    projection: camera.projection.clone(),
    node: adder(camera.node.clone()),
    attach_index: None,
  }
  .into_ptr();

  let mut previous_node = camera.node.clone();

  pass_changes_to(m, &r, move |delta| match delta {
    SceneCameraImplDelta::node(node) => {
      remover(previous_node.clone());
      previous_node = node.clone();
      SceneCameraImplDelta::node(adder(node))
    }
    _ => delta,
  });
  r
}

fn transform_light_node(m: &SceneLight, rebuilder: &ShareableRebuilder) -> SceneLight {
  let (adder, remover) = make_add_remover(rebuilder);

  let light = m.read();
  let r = SceneLightImpl {
    node: adder(light.node.clone()),
    light: light.light.clone(),
    attach_index: None,
  }
  .into_ptr();

  let mut previous_node = light.node.clone();

  pass_changes_to(m, &r, move |delta| match delta {
    SceneLightImplDelta::node(node) => {
      remover(previous_node.clone());
      previous_node = node.clone();
      SceneLightImplDelta::node(adder(node))
    }
    _ => delta,
  });
  r
}

fn transform_model_node(m: &SceneModel, rebuilder: &ShareableRebuilder) -> SceneModel {
  let (adder, remover) = make_add_remover(rebuilder);

  let model = m.read();
  let r = SceneModelImpl::new(model.model.clone(), adder(model.node.clone())).into_ptr();

  let mut previous_node = model.node.clone();

  pass_changes_to(m, &r, move |delta| match delta {
    SceneModelImplDelta::node(node) => {
      remover(previous_node.clone());
      previous_node = node.clone();
      SceneModelImplDelta::node(adder(node))
    }
    _ => delta,
  });
  r
}

type NodeArenaIndex = usize;
type NodeGuid = u64;
type SceneGuid = u64;

struct NodeMapping {
  mapped: SceneNode,
  sub_tree_entity_ref_count: usize,
}

struct SceneWatcher {
  change_remove_token: RemoveToken<TreeMutation<SceneNodeData>>,
  ref_count: usize,
  nodes: SceneNodeCollection,
}

impl Drop for SceneWatcher {
  fn drop(&mut self) {
    self.nodes.inner.source.off(self.change_remove_token);
  }
}

type ShareableRebuilder = Arc<RwLock<SceneRebuilder>>;

struct SceneRebuilder {
  // key original
  nodes: FastHashMap<NodeGuid, NodeMapping>,
  // (mapped, original)
  id_mapping: FastHashMap<(SceneGuid, NodeArenaIndex), (NodeGuid, NodeGuid)>,
  scenes: FastHashMap<SceneGuid, SceneWatcher>,
  target_collection: SceneNodeCollection,
}

impl SceneRebuilder {
  pub fn new(target_collection: SceneNodeCollection) -> Self {
    Self {
      nodes: Default::default(),
      scenes: Default::default(),
      id_mapping: Default::default(),
      target_collection,
    }
  }
}

fn add_watch_origin_scene_change(rebuilder: &ShareableRebuilder, source_node: &SceneNode) {
  let mut rebuilder_mut = rebuilder.write().unwrap();
  let scene_guid = source_node.scene_id;

  let scene_watcher = rebuilder_mut.scenes.entry(scene_guid).or_insert_with(|| {
    let source_collection = SceneNodeCollection {
      inner: source_node.inner.inner.read().unwrap().nodes.clone(),
      scene_guid,
    };
    let source_collection_c = source_collection.clone();
    let rebuilder = Arc::downgrade(rebuilder);

    let remove_token = source_node.inner.visit_raw_storage(move |tree| {
      tree.source.on(move |delta| {
        if let Some(rebuilder) = rebuilder.upgrade() {
          let mut rebuilder = rebuilder.write().unwrap();

          match delta {
            tree::TreeMutation::Attach {
              parent_target,
              node,
            } => {
              if let Some(node_guid) = rebuilder.try_get_original_node_guid(scene_guid, *node) {
                rebuilder.handle_attach(node_guid, *parent_target, &source_collection);
              }
            }
            tree::TreeMutation::Detach { node } => {
              if let Some(node_guid) = rebuilder.try_get_original_node_guid(scene_guid, *node) {
                rebuilder.handle_detach(node_guid, *node, &source_collection);
              }
            }
            tree::TreeMutation::Mutate { node, delta } => {
              // get the mapped node
              if let Some(node_guid) = rebuilder.try_get_original_node_guid(scene_guid, *node) {
                let node = &rebuilder.nodes.get(&node_guid).unwrap().mapped;
                // pass the delta
                node.mutate(|mut n| n.modify(delta.clone()))
              }
            }
            _ => {}
          }
          false
        } else {
          true
        }
      })
    });

    SceneWatcher {
      change_remove_token: remove_token,
      ref_count: 0, // will increase later
      nodes: source_collection_c,
    }
  });

  scene_watcher.ref_count += 1;
}

fn remove_watch_origin_scene_change(rebuilder: &ShareableRebuilder, node: &SceneNode) {
  let mut rebuilder_mut = rebuilder.write().unwrap();
  let scene_watcher = rebuilder_mut.scenes.get_mut(&node.scene_id).unwrap();

  assert!(scene_watcher.ref_count >= 1);
  scene_watcher.ref_count -= 1;

  if scene_watcher.ref_count == 0 {
    rebuilder_mut.scenes.remove(&node.scene_id);
  }
}

fn add_entity_used_node(rebuilder: &ShareableRebuilder, to_add_node: &SceneNode) -> SceneNode {
  let node = {
    rebuilder
      .write()
      .unwrap()
      .add_entity_used_node_impl(to_add_node)
  };
  add_watch_origin_scene_change(rebuilder, to_add_node);
  node
}

fn remove_entity_used_node(rebuilder: &ShareableRebuilder, to_remove_node: &SceneNode) {
  rebuilder
    .write()
    .unwrap()
    .remove_entity_used_node_impl(to_remove_node);
  remove_watch_origin_scene_change(rebuilder, to_remove_node)
}

impl SceneRebuilder {
  fn handle_attach(
    &mut self,
    child_guid: NodeGuid,
    parent_id: NodeArenaIndex,
    source_nodes: &SceneNodeCollection,
  ) {
    let child_sub_tree_entity_ref_count = self
      .nodes
      .get(&child_guid)
      .unwrap()
      .sub_tree_entity_ref_count;

    self.check_insert_and_update_parents_entity_ref_count(
      source_nodes,
      parent_id,
      child_sub_tree_entity_ref_count,
    );

    let parent_guid = self
      .try_get_original_node_guid(source_nodes.scene_guid, parent_id)
      .unwrap();

    let child = self.nodes.get(&child_guid).unwrap();

    let parent = self.nodes.get(&parent_guid).unwrap();
    child.mapped.attach_to(&parent.mapped).unwrap();
  }

  fn handle_detach(
    &mut self,
    node_guid: NodeGuid,
    node_id: NodeArenaIndex,
    source_nodes: &SceneNodeCollection,
  ) {
    let child = self.nodes.get(&node_guid).unwrap();
    child.mapped.detach_from_parent().unwrap();

    self.decrease_parent_chain_entity_ref_count_and_check_delete(
      source_nodes,
      node_id,
      child.sub_tree_entity_ref_count,
      false,
    );
  }

  fn check_insert_and_update_parents_entity_ref_count(
    &mut self,
    source_nodes: &SceneNodeCollection,
    node_handle: NodeArenaIndex,
    ref_add_count: usize,
  ) {
    let mut last_child = None;
    let source_scene_guid = source_nodes.scene_guid;

    visit_self_parent_chain(
      source_nodes,
      node_handle,
      |node_guid, node_id, node_data| {
        let mut new_created_parent = false;
        let NodeMapping {
          sub_tree_entity_ref_count,
          ..
        } = self.nodes.entry(node_guid).or_insert_with(|| {
          let mapped = self.target_collection.create_node(node_data.clone());

          self
            .id_mapping
            .insert((source_scene_guid, node_id), (mapped.guid(), node_guid));

          new_created_parent = true;

          NodeMapping {
            mapped,
            // will be increased later
            sub_tree_entity_ref_count: 0,
          }
        });

        *sub_tree_entity_ref_count += ref_add_count;

        if let Some(last_child) = last_child {
          let last_child = self.nodes.get(&last_child).unwrap();
          if new_created_parent || last_child.mapped.visit_parent(|_| {}).is_none() {
            let current = self.nodes.get(&node_guid).unwrap();
            last_child.mapped.attach_to(&current.mapped).unwrap();
          }
        }

        last_child = node_guid.into();
      },
    )
  }

  fn decrease_parent_chain_entity_ref_count_and_check_delete(
    &mut self,
    nodes: &SceneNodeCollection,
    node_handle: NodeArenaIndex,
    ref_decrease_count: usize,
    update_self_ref: bool,
  ) {
    let source_scene_guid = nodes.scene_guid;
    let mut is_self = true;

    visit_self_parent_chain(nodes, node_handle, |node_guid, node_id, _node_data| {
      let mapping = self.nodes.get_mut(&node_guid).unwrap();

      if update_self_ref || !is_self {
        assert!(mapping.sub_tree_entity_ref_count >= ref_decrease_count);
        mapping.sub_tree_entity_ref_count -= ref_decrease_count;
      }

      if mapping.sub_tree_entity_ref_count == 0 {
        self.nodes.remove(&node_guid);
        self.id_mapping.remove(&(source_scene_guid, node_id));
      }
      is_self = false;
    })
  }

  fn try_get_original_node_guid(
    &self,
    scene_id: SceneGuid,
    index: NodeArenaIndex,
  ) -> Option<NodeGuid> {
    self.id_mapping.get(&(scene_id, index)).map(|v| v.1)
  }

  fn add_entity_used_node_impl(&mut self, to_add_node: &SceneNode) -> SceneNode {
    let source_nodes = to_add_node.get_node_collection();
    self.check_insert_and_update_parents_entity_ref_count(
      &source_nodes,
      to_add_node.raw_handle().index(),
      1,
    );
    self.nodes.get(&to_add_node.guid()).unwrap().mapped.clone()
  }

  fn remove_entity_used_node_impl(&mut self, to_remove_node: &SceneNode) {
    let source_nodes = to_remove_node.get_node_collection();
    self.decrease_parent_chain_entity_ref_count_and_check_delete(
      &source_nodes,
      to_remove_node.raw_handle().index(),
      1,
      true,
    )
  }
}

fn visit_self_parent_chain(
  nodes: &SceneNodeCollection,
  node_handle: NodeArenaIndex,
  mut f: impl FnMut(NodeGuid, NodeArenaIndex, &SceneNodeData),
) {
  let tree = nodes.inner.inner.read().unwrap();
  let node_handle = tree.recreate_handle(node_handle);

  tree.create_node_ref(node_handle).traverse_parent(|node| {
    let data = node.node.data();
    let index = node.node.handle().index();
    f(data.guid(), index, data);
    true
  })
}
