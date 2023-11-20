use std::task::Context;

use crate::*;

// we only care about the given scene's scene models, to achieve this, simply check the scene id
// on node change, the node's scene id itself is immutable.
pub fn create_scene_node_checker(scene_id: u64) -> impl Fn(&SceneNode) -> Option<()> + Copy {
  move |node: &SceneNode| {
    if node.scene_and_node_id().0 == scene_id {
      Some(())
    } else {
      None
    }
  }
}

pub struct NodeRebuilder {
  node_source: NodeIncrementalDeriveCollections,
  node_mapping: FastHashMap<NodeIdentity, SceneNode>,
  target_scene: Scene,
}

impl NodeRebuilder {
  pub fn new(node_source: NodeIncrementalDeriveCollections, target: &Scene) -> Self {
    Self {
      node_source,
      node_mapping: Default::default(),
      target_scene: target.clone(),
    }
  }

  pub fn poll_update(&mut self, cx: &mut Context) {
    if let Poll::Ready(Some(changes)) = self.node_source.world_mat.poll_changes(cx) {
      for change in changes {
        match change {
          CollectionDelta::Delta(key, new, _) => {
            let node = self.node_mapping.entry(key).or_insert_with(|| {
              // create new node on target scene
              todo!()
            });
            // sync the node change
            todo!();
          }
          CollectionDelta::Remove(key, _) => {
            self.node_mapping.remove(&key);
            // remove node, raii drop from the target scene.
          }
        }
      }
    }
    if let Poll::Ready(Some(changes)) = self.node_source.net_visible.poll_changes(cx) {
      for change in changes {
        if let CollectionDelta::Delta(key, new, _) = change {
          let node = self.node_mapping.get(&key).unwrap();
          // sync the node change, the add remove is handled above
        }
      }
    }
  }
}

pub struct SceneCameraRebuilder {
  nodes: NodeRebuilder,
  camera_scope: Box<dyn DynamicReactiveCollection<AllocIdx<SceneCameraImpl>, ()>>,
  target_scene: Scene,
  camera_mapping: FastHashMap<AllocIdx<SceneCameraImpl>, SceneCameraHandle>,
}

impl SceneCameraRebuilder {
  pub fn new(
    source_scene_id: u64,
    camera_node_relation: impl ReactiveOneToManyRelationship<NodeIdentity, AllocIdx<SceneCameraImpl>>,
    source_scene_derives: &NodeIncrementalDeriveCollections,
    target_scene: &Scene,
  ) -> Self {
    let node_checker = create_scene_node_checker(source_scene_id);

    let referenced_camera = storage_of::<SceneCameraImpl>()
      .listen_to_reactive_collection(move |change| match change {
        incremental::MaybeDeltaRef::Delta(delta) => match delta {
          SceneCameraImplDelta::node(node) => Some(node_checker(node)),
          _ => None,
        },
        incremental::MaybeDeltaRef::All(sm) => Some(node_checker(&sm.node)),
      })
      .collective_filter_map(|v| v);

    let referenced_camera = referenced_camera.into_forker();

    let referenced_nodes = referenced_camera
      .clone()
      .many_to_one_reduce_key(camera_node_relation);

    let node_source = source_scene_derives.filter_by_keysets(referenced_nodes.into_forker());

    let nodes = NodeRebuilder::new(node_source, target_scene);

    Self {
      nodes,
      camera_scope: Box::new(referenced_camera),
      target_scene: target_scene.clone(),
      camera_mapping: Default::default(),
    }
  }

  pub fn poll_updates(&mut self, cx: &mut Context) {
    if let Poll::Ready(Some(changes)) = self.camera_scope.poll_changes(cx) {
      for change in changes {
        match change {
          CollectionDelta::Delta(key, new, _) => {
            //
          }
          CollectionDelta::Remove(key, _) => {
            //
          }
        }
      }
    }
  }
}

pub struct SceneLightsRebuilder {
  nodes: NodeRebuilder,
  light_scope: Box<dyn DynamicReactiveCollection<AllocIdx<SceneLightImpl>, ()>>,
  target_scene: Scene,
  light_mapping: FastHashMap<AllocIdx<SceneLightImpl>, SceneCameraHandle>,
}

impl SceneLightsRebuilder {
  pub fn new(
    source_scene_id: u64,
    light_node_relation: impl ReactiveOneToManyRelationship<NodeIdentity, AllocIdx<SceneLightImpl>>,
    source_scene_derives: &NodeIncrementalDeriveCollections,
    target_scene: &Scene,
  ) -> Self {
    let node_checker = create_scene_node_checker(source_scene_id);

    let referenced_lights = storage_of::<SceneLightImpl>()
      .listen_to_reactive_collection(move |change| match change {
        incremental::MaybeDeltaRef::Delta(delta) => match delta {
          SceneLightImplDelta::node(node) => Some(node_checker(node)),
          _ => None,
        },
        incremental::MaybeDeltaRef::All(sm) => Some(node_checker(&sm.node)),
      })
      .collective_filter_map(|v| v);

    let referenced_lights = referenced_lights.into_forker();

    let referenced_nodes = referenced_lights
      .clone()
      .many_to_one_reduce_key(light_node_relation);

    let node_source = source_scene_derives.filter_by_keysets(referenced_nodes.into_forker());

    let nodes = NodeRebuilder::new(node_source, target_scene);

    Self {
      nodes,
      light_scope: Box::new(referenced_lights),
      target_scene: target_scene.clone(),
      light_mapping: Default::default(),
    }
  }

  pub fn poll_updates(&mut self, cx: &mut Context) {
    if let Poll::Ready(Some(changes)) = self.light_scope.poll_changes(cx) {
      for change in changes {
        match change {
          CollectionDelta::Delta(key, new, _) => {
            //
          }
          CollectionDelta::Remove(key, _) => {
            //
          }
        }
      }
    }
  }
}
