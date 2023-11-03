use core::{
  pin::Pin,
  task::{Context, Poll},
};

use futures::*;
use reactive::{once_forever_pending, SignalStreamExt, StreamMap};
use rendiation_mesh_core::MeshDrawGroup;

mod utils;
use utils::*;

use crate::*;

const ENABLE_INSTANCE_DEBUG_LOGGING: bool = true;

macro_rules! debug_log {
  ($($e:expr),+) => {
    {
      if ENABLE_INSTANCE_DEBUG_LOGGING {
        println!($($e),+)
      }
    }
  };
}

pub struct AutoInstanceSystem {
  pub stat: TransformStat,
}

impl AutoInstanceSystem {
  pub fn new(
    scene_delta: impl Stream<Item = MixSceneDelta> + Unpin + 'static,
    d_systems: &FastHashMap<u64, SceneNodeDeriveSystem>,
  ) -> (
    Self,
    impl Stream<Item = MixSceneDelta>,
    FastHashMap<u64, SceneNodeDeriveSystem>,
  ) {
    // todo make sure the d_systems are polled
    let mut source_scene_derives = d_systems.clone();
    let mut source_scene_derives_output = d_systems.clone();
    let (output, stat, new_scene, new_derives) = model_transform(
      scene_delta,
      move |model_input, new_scene_nodes, new_derives| {
        source_scene_derives.insert(new_scene_nodes.scene_guid, new_derives.clone());
        let optimization = InstanceOptimization {
          new_nodes_storage: new_scene_nodes.clone(),
          source_scene_derives,
        };
        recyclable_hash_many_to_one(model_input, optimization)
      },
    );

    source_scene_derives_output.insert(new_scene.guid(), new_derives);

    (Self { stat }, output, source_scene_derives_output)
  }
}

#[derive(Clone)]
struct InstanceOptimization {
  new_nodes_storage: SceneNodeCollection,
  source_scene_derives: FastHashMap<u64, SceneNodeDeriveSystem>,
}

impl RecyclableHashManyToOne for InstanceOptimization {
  type Transformer = Transformer;
  type Key = PossibleInstanceKey;

  fn create_key(&self, model: &SceneModel) -> Self::Key {
    let derives = self
      .source_scene_derives
      .get(&model.read().node.scene_id)
      .unwrap();
    compute_instance_key(model, derives)
  }

  fn create_transformer(&self, key: Self::Key) -> Self::Transformer {
    Transformer::new(
      key,
      self.source_scene_derives.clone(),
      self.new_nodes_storage.clone(),
    )
  }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct InstanceKey {
  pub is_front_side: bool,
  pub content: InstanceContentKey,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct InstanceContentKey {
  pub material_id: u64,
  pub mesh_id: u64,
  pub group: MeshDrawGroup,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
enum PossibleInstanceKey {
  UnableToInstance(u64), // just the origin model uuid
  Instanced(InstanceKey),
}

enum InstanceSourceIncrementalUpdate {
  WorldMat(Mat4<f32>),
  Visibility,
  InstanceKeyChanged,
}

fn is_front_side(mat: Mat4<f32>) -> bool {
  mat.det().is_sign_positive()
}

fn compute_instance_key(model: &SceneModel, d_sys: &SceneNodeDeriveSystem) -> PossibleInstanceKey {
  let m = model.read();
  let content = match &m.model {
    ModelEnum::Standard(model) => compute_instance_key_inner(model),
    ModelEnum::Foreign(_) => None,
  };

  if let Some(content) = content {
    let mat = d_sys.get_world_matrix(&m.node);
    let instance_key = InstanceKey {
      is_front_side: is_front_side(mat),
      content,
    };

    PossibleInstanceKey::Instanced(instance_key)
  } else {
    PossibleInstanceKey::UnableToInstance(m.guid())
  }
}

/// for materials not suitable for instance, material should impl and register this to rule out
pub trait InstanceSourceRuleOut {}
define_dyn_trait_downcaster_static!(InstanceSourceRuleOut);

fn compute_instance_key_inner(
  model: &IncrementalSignalPtr<StandardModel>,
) -> Option<InstanceContentKey> {
  let model = model.read();

  if let MaterialEnum::Foreign(m) = &model.material {
    if get_dyn_trait_downcaster_static!(InstanceSourceRuleOut)
      .downcast_ref(m.as_ref().as_any())
      .is_some()
    {
      return None;
    }
  }

  if let MeshEnum::TransformInstanced(_) = &model.mesh {
    return None;
  }

  InstanceContentKey {
    material_id: model.material.guid()?,
    mesh_id: model.mesh.guid()?,
    group: model.group,
  }
  .into()
}

/// we call it transformer here because maybe this struct will likely be reused in another optimizer
#[pin_project::pin_project]
struct Transformer {
  d_sys: FastHashMap<u64, SceneNodeDeriveSystem>,
  new_nodes: SceneNodeCollection,
  key: PossibleInstanceKey,
  #[pin]
  source: StreamMap<u64, InstanceSourceStream>,
  source_model: FastHashMap<u64, SceneModel>,
  removals: Vec<u64>,
  transformed: Option<(SceneModel, bool)>,
}

impl Transformer {
  pub fn new(
    key: PossibleInstanceKey,
    d_sys: FastHashMap<u64, SceneNodeDeriveSystem>,
    new_nodes: SceneNodeCollection,
  ) -> Self {
    debug_log!("create new transformer with key: {key:?}");
    Self {
      key,
      d_sys,
      new_nodes,
      source: Default::default(),
      source_model: Default::default(),
      removals: Default::default(),
      transformed: Default::default(),
    }
  }
}

impl ModelProxy for Transformer {
  fn insert_source_model(&mut self, source: SceneModel) {
    let derives = self.d_sys.get(&source.read().node.scene_id).unwrap();
    let change = build_instance_source_stream(&source, derives, self.key.clone());
    self.source.insert(source.guid(), change);
    self.source_model.insert(source.guid(), source);
  }

  fn remove_source_model_by_guid(&mut self, source_id: u64) {
    let _ = self.source.remove(source_id).unwrap();
    self.removals.push(source_id);
    // note, we do not remove the source_model map here, the stream polling will do this
    // automatically
  }
}

impl Stream for Transformer {
  type Item = Vec<TransformerDelta>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    // we simply recreate new instances if any incremental source changed (could optimize later)
    let mut batched = ready!(this.source.poll_next_unpin(cx)).unwrap(); // unwrap is safe

    if batched.is_empty() && this.source_model.is_empty() {
      debug_assert!(this.transformed.is_none());
      debug_log!("transformer_drop, key: {:?}", this.key);
      return Poll::Ready(None);
    }

    let mut results = Vec::new();
    let mut require_rebuild = false;
    let mut to_recycle = Vec::<SceneModel>::new();
    let mut to_remove = Vec::new();

    this.removals.drain(..).for_each(|idx| {
      to_remove.push(this.source_model.get(&idx).unwrap().clone());
      require_rebuild = true
    });

    batched.drain(..).for_each(|d| match d {
      // handled insert here because we do not have any extra state to record insertion on the
      // transformer
      reactive::StreamMapDelta::Insert(_) => require_rebuild = true,
      reactive::StreamMapDelta::Remove(_) => {}
      reactive::StreamMapDelta::Delta(idx, d) => match d {
        InstanceSourceIncrementalUpdate::WorldMat(_) => {
          // should optimize later
          require_rebuild = true;
        }
        InstanceSourceIncrementalUpdate::InstanceKeyChanged => {
          let m = this.source_model.get(&idx).unwrap().clone();
          if !to_recycle.iter().any(|model| model.guid() == m.guid()) {
            to_recycle.push(m);
          }
          require_rebuild = true;
        }
        InstanceSourceIncrementalUpdate::Visibility => {
          // should optimize later
          require_rebuild = true;
        }
      },
    });

    to_remove
      .iter()
      .map(|m| m.guid())
      .chain(to_recycle.iter().map(|m| m.guid()))
      .for_each(|idx| {
        this.source_model.remove(&idx).unwrap();
      });

    // source model could be removed to empty, but we should not return Poll::Ready None here
    // because to_recycle list maybe contains stuff

    // recycle minus drop
    to_remove.iter().for_each(|m| {
      if let Some(to_recycle_but_dropped) =
        to_recycle.iter().position(|model| model.guid() == m.guid())
      {
        to_recycle.remove(to_recycle_but_dropped);
      }
    });

    to_recycle.iter().for_each(|model| {
      this.source.remove(model.guid());
    });

    use TransformerDelta::*;
    results.extend(to_remove.into_iter().map(DropSource));
    results.extend(to_recycle.into_iter().map(ReleaseUnsuitable));

    if require_rebuild {
      if let Some((old, old_is_instance)) = this.transformed.take() {
        if old_is_instance {
          debug_log!("remove transformed model guid: {}", old.guid());
        } else {
          debug_log!("remove original model guid: {}", old.guid());
        }
        results.push(TransformerDelta::RemoveTransformed(old, old_is_instance));
      }

      if let Some((new_transformed, is_instance)) =
        create_instance(this.source_model, this.d_sys, this.new_nodes)
      {
        if is_instance {
          debug_log!(
            "created transformed model guid: {}, instance count {}",
            new_transformed.guid(),
            this.source_model.len()
          );
        } else {
          debug_log!("created original model guid: {}", new_transformed.guid());
        }
        results.push(TransformerDelta::NewTransformed(
          new_transformed.clone(),
          is_instance,
        ));
        *this.transformed = (new_transformed, is_instance).into();
      }
    }

    if results.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(results.into())
    }
  }
}

type InstanceSourceStream = impl Stream<Item = InstanceSourceIncrementalUpdate> + Unpin;
type BoxedWatcher = Box<dyn Stream<Item = InstanceSourceIncrementalUpdate> + Unpin>;

// watch a model, check if the model's instance key matches the key passed in
// and return the stream of InstanceSourceIncrementalUpdate
fn build_instance_source_stream(
  model: &SceneModel,
  d: &SceneNodeDeriveSystem,
  key: PossibleInstanceKey,
) -> InstanceSourceStream {
  use InstanceSourceIncrementalUpdate as InsUpdate;
  use PossibleInstanceKey as PIK;
  let d = d.clone();

  let is_font_side = if let PIK::Instanced(key) = &key {
    key.is_front_side.into()
  } else {
    None
  };

  let world_matrix = model
    .single_listen_by(with_field!(SceneModelImpl => node))
    .filter_map_sync(move |n| d.create_derive_stream(&n))
    .flatten_signal()
    .filter_map_sync(move |delta| match delta {
      SceneNodeDerivedDataDelta::world_matrix(mat) => is_font_side.map(|is_font_side| {
        if is_front_side(mat) == is_font_side {
          InsUpdate::WorldMat(mat)
        } else {
          InsUpdate::InstanceKeyChanged
        }
      }),
      SceneNodeDerivedDataDelta::net_visible(_) => InsUpdate::Visibility.into(),
      _ => None,
    });

  let model = model
    .single_listen_by(with_field!(SceneModelImpl => model))
    .map(move |model| match model {
      ModelEnum::Standard(sm) => {
        let model_ref = sm.downgrade();
        let key = key.clone();
        let watch = sm.unbound_listen_by(all_delta).filter_map_sync(move |_| {
          if let Some(model_ref) = model_ref.upgrade() {
            // just recompute everything
            let new_key = compute_instance_key_inner(&model_ref);
            match (new_key, &key) {
              (None, PIK::Instanced(_)) => InsUpdate::InstanceKeyChanged.into(),
              (Some(new_key), PIK::Instanced(key)) => {
                if new_key != key.content {
                  InsUpdate::InstanceKeyChanged.into()
                } else {
                  None
                }
              }
              _ => None,
            }
          } else {
            None
          }
        });

        Box::new(watch) as BoxedWatcher
      }
      ModelEnum::Foreign(_) => match key {
        PIK::UnableToInstance(_) => Box::new(futures::stream::pending()) as BoxedWatcher,
        PIK::Instanced(_) => Box::new(once_forever_pending(InsUpdate::InstanceKeyChanged)),
      },
    })
    .flatten_signal();

  futures::stream::select(world_matrix, model)
}

/// maybe failed, if the source is empty
fn create_instance(
  source: &FastHashMap<u64, SceneModel>,
  d_sys: &FastHashMap<u64, SceneNodeDeriveSystem>,
  new_nodes: &SceneNodeCollection,
) -> Option<(SceneModel, bool)> {
  let first = source.values().next()?;
  // if the source is a single model, then the transformed model is the same source model
  if source.len() == 1 {
    (first.clone(), false).into()
  } else {
    let first = first.read();
    let model = match &first.model {
      ModelEnum::Standard(model) => model,
      ModelEnum::Foreign(_) => unreachable!(),
    };

    let model = model.read();
    let mesh = model.mesh.clone();
    let material = model.material.clone();

    let transforms = source
      .values()
      .map(|m| {
        let node = &m.read().node;
        let derives = d_sys.get(&node.scene_id).unwrap();
        let mat = derives.get_world_matrix(node);
        let net_visible = derives.get_net_visible(node);
        if net_visible {
          mat
        } else {
          Mat4::zero()
        }
      })
      .collect();

    let instance_mesh = TransformInstancedSceneMesh { mesh, transforms }.into_ptr();

    let instance_model = StandardModel {
      material,
      mesh: MeshEnum::TransformInstanced(instance_mesh),
      group: model.group,
      skeleton: None,
    }
    .into_ptr();

    let instance_model = SceneModelImpl {
      model: ModelEnum::Standard(instance_model),
      node: new_nodes.create_node(Default::default()),
      attach_index: None,
    }
    .into_ptr();

    (instance_model, true).into()
  }
}
