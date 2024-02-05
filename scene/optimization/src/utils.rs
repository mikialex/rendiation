use std::task::{Context, Poll};

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
    if let Poll::Ready(changes) = self.node_source.world_mat.poll_changes(cx) {
      for (key, change) in changes.iter_key_value() {
        match change {
          ValueChange::Delta(new, _) => {
            let node = self
              .node_mapping
              .entry(key) // create new node on target scene
              .or_insert_with(|| self.target_scene.create_root_child());
            // sync the node change
            node.set_local_matrix(new);
          }
          ValueChange::Remove(_) => {
            self.node_mapping.remove(&key);
            // remove node, raii drop from the target scene.
          }
        }
      }
    }
    if let Poll::Ready(changes) = self.node_source.net_visible.poll_changes(cx) {
      for (key, change) in changes.iter_key_value() {
        // sync the node change, the add remove is handled above
        if let ValueChange::Delta(new, _) = change {
          let node = self.node_mapping.get(&key).unwrap();
          node.set_visible(new)
        }
      }
    }
  }
}

pub struct SceneCameraRebuilder {
  nodes: NodeRebuilder,
  camera_scope: Box<dyn ReactiveCollection<AllocIdx<SceneCameraImpl>, ()>>,
  target_scene: Scene,
  camera_mapping: FastHashMap<AllocIdx<SceneCameraImpl>, SceneCameraHandle>,
}

impl SceneCameraRebuilder {
  pub fn new(
    source_scene_id: u64,
    source_scene_derives: &NodeIncrementalDeriveCollections,
    target_scene: &Scene,
  ) -> Self {
    let node_checker = create_scene_node_checker(source_scene_id);

    let referenced_camera =
      storage_of::<SceneCameraImpl>().listen_to_reactive_collection(move |change| match change {
        incremental::MaybeDeltaRef::Delta(delta) => match delta {
          SceneCameraImplDelta::node(node) => ChangeReaction::Care(node_checker(node)),
          _ => ChangeReaction::NotCare,
        },
        incremental::MaybeDeltaRef::All(sm) => ChangeReaction::Care(node_checker(&sm.node)),
      });

    let referenced_camera = referenced_camera.into_forker();

    let referenced_nodes = referenced_camera
      .clone()
      .many_to_one_reduce_key(scene_camera_ref_node_many_one_relation());

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
    self.nodes.poll_update(cx);
    if let Poll::Ready(changes) = self.camera_scope.poll_changes(cx) {
      let cameras_storage = storage_of::<SceneCameraImpl>();
      let cameras = cameras_storage.inner.data.read_recursive();

      // copy the source by full delta
      let mut to_sync_target = Vec::new();
      let mut to_sync_delta = Vec::new();
      for (key, change) in changes.iter_key_value() {
        match change {
          ValueChange::Delta(_, _) => {
            let camera = &cameras.get(key.index).data;

            let offset = to_sync_delta.len();
            camera.expand_push_into(&mut to_sync_delta);
            let offset_2 = to_sync_delta.len();

            to_sync_target.push((key, offset, offset_2));
          }
          ValueChange::Remove(_) => {
            let in_target_scene = self.camera_mapping.remove(&key).unwrap();
            self.target_scene.remove_camera(in_target_scene);
          }
        }
      }
      drop(cameras);
      drop(cameras_storage);

      // sync the change
      for (target, offset_start, offset_end) in to_sync_target {
        let camera_handle = self.camera_mapping.entry(target).or_insert_with(|| {
          // create default
          let root = self.target_scene.root();
          let perspective = CameraProjectionEnum::Perspective(Default::default());
          let camera = SceneCameraImpl::new(perspective, root).into_ptr();
          self.target_scene.insert_camera(camera)
        });

        let scene_c = self.target_scene.read();
        let scene_cc = scene_c.core.read();
        let camera = scene_cc.cameras.get(*camera_handle).unwrap();
        for delta in to_sync_delta.get(offset_start..offset_end).unwrap() {
          camera.mutate(|mut m| m.modify(delta.clone()))
        }
      }
    }
  }
}

pub struct SceneLightsRebuilder {
  nodes: NodeRebuilder,
  light_scope: Box<dyn ReactiveCollection<AllocIdx<SceneLightImpl>, ()>>,
  target_scene: Scene,
  light_mapping: FastHashMap<AllocIdx<SceneLightImpl>, SceneLightHandle>,
}

impl SceneLightsRebuilder {
  pub fn new(
    source_scene_id: u64,
    source_scene_derives: &NodeIncrementalDeriveCollections,
    target_scene: &Scene,
  ) -> Self {
    let node_checker = create_scene_node_checker(source_scene_id);

    let referenced_lights =
      storage_of::<SceneLightImpl>().listen_to_reactive_collection(move |change| match change {
        incremental::MaybeDeltaRef::Delta(delta) => match delta {
          SceneLightImplDelta::node(node) => ChangeReaction::Care(node_checker(node)),
          _ => ChangeReaction::NotCare,
        },
        incremental::MaybeDeltaRef::All(sm) => ChangeReaction::Care(node_checker(&sm.node)),
      });

    let referenced_lights = referenced_lights.into_forker();

    let referenced_nodes = referenced_lights
      .clone()
      .many_to_one_reduce_key(scene_light_ref_node_many_one_relation());

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
    self.nodes.poll_update(cx);
    if let Poll::Ready(changes) = self.light_scope.poll_changes(cx) {
      let lights_storage = storage_of::<SceneLightImpl>();
      let lights = lights_storage.inner.data.read_recursive();

      // copy the source by full delta
      let mut to_sync_target = Vec::new();
      let mut to_sync_delta = Vec::new();
      for (key, change) in changes.iter_key_value() {
        match change {
          ValueChange::Delta(_, _) => {
            let light = &lights.get(key.index).data;

            let offset = to_sync_delta.len();
            light.expand_push_into(&mut to_sync_delta);
            let offset_2 = to_sync_delta.len();

            to_sync_target.push((key, offset, offset_2));
          }
          ValueChange::Remove(_) => {
            let in_target_scene = self.light_mapping.remove(&key).unwrap();
            self.target_scene.remove_light(in_target_scene);
          }
        }
      }
      drop(lights);
      drop(lights_storage);

      // sync the change
      for (target, offset_start, offset_end) in to_sync_target {
        let light_handle = self.light_mapping.entry(target).or_insert_with(|| {
          // create default
          let root = self.target_scene.root();
          let perspective = LightEnum::Foreign(Box::new(()));
          let light = SceneLightImpl::new(perspective, root).into_ptr();
          self.target_scene.insert_light(light)
        });

        let scene_c = self.target_scene.read();
        let scene_cc = scene_c.core.read();
        let light = scene_cc.lights.get(*light_handle).unwrap();
        for delta in to_sync_delta.get(offset_start..offset_end).unwrap() {
          light.mutate(|mut m| m.modify(delta.clone()))
        }
      }
    }
  }
}
