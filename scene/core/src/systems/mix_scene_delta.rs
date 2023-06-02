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
  let nodes_holder: Arc<RwLock<SceneRebuilder>> = Default::default();
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
          remove_entity_used_node(&mut nodes_holder.write().unwrap(), &model.read().node);
          model_handle_map.remove(&model.guid()).unwrap();
        }
        ContainerRefRetainContentDelta::Insert(model) => {
          let holder = nodes_holder.clone();
          let holder2 = nodes_holder.clone();
          let nodes = nodes.clone();
          let nodes2 = nodes.clone();
          let new_model = transform_model_node2(
            model,
            move |node| add_entity_used_node(&mut holder.write().unwrap(), node),
            move |node| remove_entity_used_node(&mut holder2.write().unwrap(), node),
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

#[derive(Default)]
struct SceneRebuilder {
  nodes: HashMap<usize, NodeMapping>,
  scenes: HashMap<usize, SceneWatcher>,
  target_collection: SceneNodeCollection,
}

fn visit_self_parent_chain(
  nodes: &SceneNodeCollection,
  node_handle: usize,
  f: impl FnMut(usize, &mut SceneNodeDataImpl),
) {
  let tree = nodes.inner.inner.read().unwrap();
  // tree.inner.create_node_ref(handle)
}

impl SceneRebuilder {
  fn add_watch_scene_structure_change(&mut self, node: &SceneNode) {
    node.inner.visit_raw_storage(|tree| {
      tree.source.on(|delta| {
        match delta {
          tree::TreeMutation::Attach {
            parent_target,
            node,
          } => {
            let parent_guid = todo!();
            let node_guid = todo!();
          }
          tree::TreeMutation::Detach { node } => {
            let node_to_detach = todo!();
          }
          tree::TreeMutation::Mutate { node, delta } => {
            let node_to_detach = todo!();
          }
          _ => {}
        }
        //
        false
      });
    });
  }
  fn remove_watch_scene_structure_change(&mut self, node: &SceneNode) {
    //
  }

  fn handle_attach(
    &mut self,
    child_node_guid: usize,
    parent_guid: usize,
    source_nodes: &SceneNodeCollection,
  ) {
    let child = self.nodes.get(&child_node_guid).unwrap();
    self.check_insert_and_update_parents_entity_ref_count(
      source_nodes,
      parent_guid,
      child.sub_tree_entity_ref_count,
    );
    let parent = self.nodes.get(&parent_guid).unwrap();
    child.mapped.attach_to(&parent.mapped);
    // todo visit parent chain, add ref count
  }

  fn handle_detach(&mut self, node_guid: usize) {
    let child = self.nodes.get(&node_guid).unwrap();
    child.mapped.detach_from_parent();
    // todo, visit parent chain, decrease ref count, remove node.
  }

  fn check_insert_and_update_parents_entity_ref_count(
    &mut self,
    source_nodes: &SceneNodeCollection,
    node_handle: usize,
    ref_add_count: usize,
  ) {
    let mut child_to_attach = None;
    visit_self_parent_chain(source_nodes, node_handle, |node_guid, node_data| {
      let NodeMapping {
        sub_tree_entity_ref_count,
        ..
      } = self.nodes.entry(node_guid).or_insert_with(|| {
        let mapped = self.target_collection.create_node(node_data.clone());
        child_to_attach = Some(node_guid);
        NodeMapping {
          mapped,
          sub_tree_entity_ref_count: 0, // will be add later
          origin_guid: node_guid,
          origin_scene_id: todo!(),
          origin_scene_handle_index: todo!(),
        }
      });

      *sub_tree_entity_ref_count += ref_add_count;

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

  fn decrease_parent_chain_entity_ref_count_and_check_delete(
    &mut self,
    nodes: &SceneNodeCollection,
    node_handle: usize,
    ref_decrease_count: usize,
  ) {
    visit_self_parent_chain(nodes, node_handle, |node_guid, _node_data| {
      let (mapped_node, ref_count) = self.nodes.get_mut(&node_guid).unwrap();
      *ref_count -= ref_decrease_count;
      assert!(*ref_count >= 0);
      if *ref_count == 0 {
        self.nodes.remove(&node_guid);
      }
    })
  }

  fn add_entity_used_node(&mut self, to_add_node: &SceneNode) -> SceneNode {
    let source_nodes = to_add_node.get_node_collection();
    self.check_insert_and_update_parents_entity_ref_count(
      &source_nodes,
      to_add_node.raw_handle().index(),
      1,
    );
    self.nodes.get(&to_add_node.guid()).unwrap().mapped.clone()
  }

  fn remove_entity_used_node(&mut self, to_remove_node: &SceneNode) {
    let source_nodes = to_remove_node.get_node_collection();
    self.decrease_parent_chain_entity_ref_count_and_check_delete(
      &source_nodes,
      to_remove_node.raw_handle().index(),
      1,
    )
  }
}
