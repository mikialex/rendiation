use futures::StreamExt;
use reactive::RemoveToken;

use crate::*;

/// compare to scene inner delta, this mixed delta support multi scene content mixing
#[derive(Clone)]
#[allow(non_camel_case_types)]
pub enum MixSceneDelta {
  background(Option<SceneBackGround>),
  default_camera(SceneCamera),
  active_camera(Option<SceneCamera>),
  cameras(ContainerRefRetainContentDelta<SceneCamera>),
  lights(ContainerRefRetainContentDelta<SceneLight>),
  models(ContainerRefRetainContentDelta<SceneModel>),
  ext(DeltaOf<DynamicExtension>),
}

// pub fn map_scene_delta_to_mixed(
//   input: impl Stream<Item = SceneInnerDelta>,
// ) -> impl Stream<Item = MixSceneDelta> {
//   //
// }

pub fn mix_scene_folding(
  input: impl Stream<Item = MixSceneDelta>,
) -> (
  impl Stream<Item = MixSceneDelta>,
  (Scene, SceneNodeDeriveSystem),
) {
  let (scene, derives) = SceneInner::new();

  let s = scene.clone();
  let nodes = scene.read().nodes.clone();
  let nodes_holder: Arc<RwLock<HashMap<usize, SceneNode>>> = Default::default();
  let mut model_handle_map: HashMap<usize, SceneModelHandle> = HashMap::new();

  let output = input.map(move |delta| {
    //
    match &delta {
      MixSceneDelta::background(_) => todo!(),
      MixSceneDelta::default_camera(_) => todo!(),
      MixSceneDelta::active_camera(_) => todo!(),
      MixSceneDelta::cameras(_) => todo!(),
      MixSceneDelta::lights(_) => todo!(),
      MixSceneDelta::models(model) => match model {
        ContainerRefRetainContentDelta::Remove(model) => {
          remove_entity_used_node(
            &mut nodes_holder.write().unwrap(),
            &nodes,
            &model.read().node,
          );
          model_handle_map.remove(&model.guid()).unwrap();
        }
        ContainerRefRetainContentDelta::Insert(model) => {
          let holder = nodes_holder.clone();
          let holder2 = nodes_holder.clone();
          let nodes = nodes.clone();
          let nodes2 = nodes.clone();
          let new_model = transform_model_node2(
            model,
            move |node| add_entity_used_node(&mut holder.write().unwrap(), &nodes, node),
            move |node| remove_entity_used_node(&mut holder2.write().unwrap(), &nodes2, node),
          );
          let new_model_handle = s.insert_model(new_model);
          model_handle_map.insert(model.guid(), new_model_handle);
        }
      },
      MixSceneDelta::ext(_) => todo!(),
    }

    delta
  });

  (output, (scene, derives))
}

pub fn transform_model_node2(
  m: &SceneModel,
  adder: impl Fn(&SceneNode) -> SceneNode + Send + Sync + 'static,
  remover: impl Fn(&SceneNode) + Send + Sync + 'static,
) -> SceneModel {
  let model = m.read();
  let r = SceneModelImpl {
    node: adder(&model.node),
    model: model.model.clone(),
  }
  .into_ref();

  let mut previous_node = model.node.clone();

  m.pass_changes_to(&r, move |delta| match delta {
    SceneModelImplDelta::node(node) => {
      remover(&previous_node);
      previous_node = node.clone();
      SceneModelImplDelta::node(adder(&node))
    }
    _ => delta,
  });
  r
}

struct NodeMapping {
  mapped: SceneNode,
  sub_tree_entity_ref_count: usize,
  origin_guid: usize,
  origin_scene_id: usize,
  origin_scene_handle_index: usize,
}

struct SceneWatcher {
  change_remove_token: RemoveToken<usize>,
  mapped_node_count: usize,
  nodes: SceneNodeCollection, // todo weak?
}

struct SceneRebuilder {
  nodes: HashMap<usize, (SceneNode, usize)>,
  scenes: HashMap<usize, SceneWatcher>,
  collection: SceneNodeCollection,
}

fn visit_self_parent_chain(node: &SceneNode, f: impl FnMut(usize, &mut SceneNodeDataImpl)) {
  node.inner.visit_raw_storage(|tree| {
    //
  })
}

impl SceneRebuilder {
  fn add_watch_scene_structure_change(&mut self, node: &SceneNode) {
    node.inner.visit_raw_storage(|tree| {
      tree.source.on(|delta| {
        //
        false
      });
    });
  }
  fn remove_watch_scene_structure_change(&mut self, node: &SceneNode) {
    //
  }

  fn handle_attach(node_with_new_parent: usize) {
    //
  }

  fn handle_detach() {
    //
  }

  pub fn insert(&mut self, node: &SceneNode) {
    let mut child_to_attach = None;
    visit_self_parent_chain(node, |node_guid, node_data| {
      let (_, ref_count) = self.nodes.entry(node_guid).or_insert_with(|| {
        let node = self.collection.create_node(node_data.clone());
        child_to_attach = Some(node_guid);
        (node, 0) // will be add later
      });
      *ref_count += 1;
      if let Some(child) = child_to_attach.take() {
        let (child, child_ref_count) = self.nodes.get(&child).unwrap();
        let child = child.clone();
        let child_ref_count = *child_ref_count;

        let (mapped_node, ref_count) = self.nodes.get_mut(&node_guid).unwrap();
        child.attach_to(mapped_node);
        *ref_count += child_ref_count;
      }
    })
  }

  pub fn remove(&mut self, node: &SceneNode) {
    visit_self_parent_chain(node, |node_guid, _node_data| {
      let (mapped_node, ref_count) = self.nodes.get_mut(&node_guid).unwrap();
      *ref_count -= 1;
      if *ref_count == 0 {
        self.nodes.remove(&node_guid);
      }
    })
  }
}

fn add_entity_used_node(
  nodes_holder: &mut HashMap<usize, SceneNode>,
  target: &SceneNodeCollection,
  to_add_node: &SceneNode,
) -> SceneNode {
  todo!()
}

fn remove_entity_used_node(
  nodes_holder: &mut HashMap<usize, SceneNode>,
  target: &SceneNodeCollection,
  to_remove_node: &SceneNode,
) {
  //
}
