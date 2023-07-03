use core::{
  pin::Pin,
  task::{Context, Poll},
};

use futures::*;
use reactive::{do_updates_by, once_forever_pending, SignalStreamExt, StreamMap, StreamMapDelta};
use rendiation_renderable_mesh::MeshDrawGroup;

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
  // maybe add some metrics collecting logic here?
}

// input
impl AutoInstanceSystem {
  // note, we have a subtle requirement that the other change in the stream has no dependency on
  // model change in the same stream or we will have to handle it manually.
  pub fn new(
    scene_delta: impl Stream<Item = MixSceneDelta> + Unpin,
    d_system: &SceneNodeDeriveSystem,
  ) -> (Self, impl Stream<Item = MixSceneDelta>) {
    let broad_cast = scene_delta.create_broad_caster();

    // split the model stream, maintain the old arena relationship
    let model_input = broad_cast
      .fork_stream()
      .filter_map_sync(|delta| match delta {
        MixSceneDelta::models(d) => Some(d),
        _ => None,
      });

    let (new_scene, _new_derives) = SceneImpl::new();
    let middle_scene_nodes = new_scene.read().core.read().nodes.clone();

    let transformed_models = instance_transform(model_input, d_system, &middle_scene_nodes)
      .map(|v| match v {
        ContainerRefRetainContentDelta::Remove((v, _)) => ContainerRefRetainContentDelta::Remove(v),
        ContainerRefRetainContentDelta::Insert((v, _)) => ContainerRefRetainContentDelta::Insert(v),
      })
      .map(MixSceneDelta::models);

    // the other change stream
    let other_stuff = broad_cast
      .fork_stream()
      .filter_map_sync(|delta| match &delta {
        MixSceneDelta::models(_) => None,
        _ => Some(delta),
      });

    let output = futures::stream::select_with_strategy(other_stuff, transformed_models, prior_left);

    (Self {}, output)
  }
}

type OriginModelId = usize;
type ModelChange = ContainerRefRetainContentDelta<SceneModel>;
type ModelOutChange = ContainerRefRetainContentDelta<(SceneModel, bool)>;

fn prior_left(_: &mut ()) -> stream::PollNext {
  stream::PollNext::Left
}

fn instance_transform(
  input: impl Stream<Item = ModelChange>,
  d_sys: &SceneNodeDeriveSystem,
  new_nodes: &SceneNodeCollection,
) -> impl Stream<Item = ModelOutChange> {
  // origin model id => transformed id
  let mut source_id_transformer_map: HashMap<OriginModelId, PossibleInstanceKey> = HashMap::new();

  // transformed id => transformed
  let transformers: StreamMap<PossibleInstanceKey, Transformer> = StreamMap::default();

  let (recycling_sender, recycled_models) = futures::channel::mpsc::unbounded();

  let input = futures::stream::select_with_strategy(
    recycled_models.map(ModelChange::Insert),
    input,
    prior_left, // always drain recycled first, because message order matters.
  );

  let d_sys = d_sys.clone();
  let new_nodes = new_nodes.clone();
  input
    .fold_signal_state_stream(transformers, move |d, transformers| {
      match d {
        ModelChange::Insert(model) => {
          let idx = model.guid();
          // for any new coming model, calculate the instance key, and find which existing instance
          // could be merged with
          let key = compute_instance_key(&model, &d_sys);
          source_id_transformer_map.insert(idx, key.clone());

          // merge into the transformer or create a new transformer
          transformers
            .get_or_insert_with(key.clone(), || {
              Transformer::new(key.clone(), d_sys.clone(), new_nodes.clone())
            })
            .add_new_source(model);
        }
        ModelChange::Remove(model) => {
          let idx = model.guid();
          let key = source_id_transformer_map.remove(&idx).unwrap();

          let transformer = transformers.get_mut(&key).unwrap();
          // remove the source model from the inside of the transformer,
          // drop the source and eventually drop the transformer if no more source in it
          transformer.notify_source_dropped(idx);
        }
      }
    })
    .filter_map_sync(|delta| {
      match delta {
        StreamMapDelta::Delta(key, deltas) => (key, deltas).into(),
        _ => None, // we do not care the transformer insertion or removal here
      }
    })
    .map(move |(_, deltas)| {
      let transform_change = deltas
        .into_iter()
        .filter_map(|delta| {
          match delta {
            TransformerDelta::ReleaseUnsuitable(source) => {
              recycling_sender.unbounded_send(source).ok();
              return None;
            }
            TransformerDelta::DropSource(_) => return None,
            TransformerDelta::NewTransformed(transformed, is_ins) => {
              ModelOutChange::Insert((transformed, is_ins))
            }
            TransformerDelta::RemoveTransformed(transformed, is_ins) => {
              ModelOutChange::Remove((transformed, is_ins))
            }
          }
          .into()
        })
        .collect::<Vec<_>>(); // we have to do a collecting because this iter borrows the recycling_sender

      futures::stream::iter(transform_change)
    })
    .flatten()
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct InstanceKey {
  pub is_front_side: bool,
  pub content: InstanceContentKey,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct InstanceContentKey {
  pub material_id: usize,
  pub mesh_id: usize,
  pub group: MeshDrawGroup,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
enum PossibleInstanceKey {
  UnableToInstance(usize), // just the origin model uuid
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
    ModelType::Standard(model) => compute_instance_key_inner(model),
    ModelType::Foreign(_) => None,
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

fn compute_instance_key_inner(model: &SceneItemRef<StandardModel>) -> Option<InstanceContentKey> {
  let model = model.read();

  if let SceneMaterialType::Foreign(m) = &model.material {
    if get_dyn_trait_downcaster_static!(InstanceSourceRuleOut)
      .downcast_ref(m.as_ref())
      .is_some()
    {
      return None;
    }
  }

  if let SceneMeshType::TransformInstanced(_) = &model.mesh {
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
  d_sys: SceneNodeDeriveSystem,
  new_nodes: SceneNodeCollection,
  key: PossibleInstanceKey,
  #[pin]
  source: StreamMap<usize, InstanceSourceStream>,
  source_model: HashMap<usize, SceneModel>,
  removals: Vec<usize>,
  transformed: Option<(SceneModel, bool)>,
}

impl Transformer {
  pub fn new(
    key: PossibleInstanceKey,
    d_sys: SceneNodeDeriveSystem,
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

  fn add_new_source(&mut self, source: SceneModel) {
    let change = build_instance_source_stream(&source, &self.d_sys, self.key.clone());
    self.source.insert(source.guid(), change);
    self.source_model.insert(source.guid(), source);
  }

  fn notify_source_dropped(&mut self, source_id: usize) {
    let _ = self.source.remove(source_id).unwrap();
    self.removals.push(source_id);
    // note, we do not remove the source_model map here, the stream polling will do this
    // automatically
  }
}

/// we only care about the reference change here(create new transformed instance)
/// the downstream could listen the new ref to get what they want.
pub enum TransformerDelta {
  ReleaseUnsuitable(SceneModel), // original model
  DropSource(SceneModel),        // original model
  NewTransformed(SceneModel, bool),
  RemoveTransformed(SceneModel, bool),
}

impl Stream for Transformer {
  type Item = Vec<TransformerDelta>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    // we simply recreate new instances if any incremental source changed (could optimize later)
    // so, here we do some batch processing to avoid unnecessary instance rebuild
    let mut batched = Vec::<_>::new();
    do_updates_by(&mut this.source, cx, |d| batched.push(d));

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
    });

  let model = model
    .single_listen_by(with_field!(SceneModelImpl => model))
    .map(move |model| match model {
      ModelType::Standard(sm) => {
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
      ModelType::Foreign(_) => match key {
        PIK::UnableToInstance(_) => Box::new(futures::stream::pending()) as BoxedWatcher,
        PIK::Instanced(_) => Box::new(once_forever_pending(InsUpdate::InstanceKeyChanged)),
      },
    })
    .flatten_signal();

  futures::stream::select(world_matrix, model)
}

/// maybe failed, if the source is empty
fn create_instance(
  source: &HashMap<usize, SceneModel>,
  d_sys: &SceneNodeDeriveSystem,
  new_nodes: &SceneNodeCollection,
) -> Option<(SceneModel, bool)> {
  let first = source.values().next()?;
  // if the source is a single model, then the transformed model is the same source model
  if source.len() == 1 {
    (first.clone(), false).into()
  } else {
    let first = first.read();
    let model = match &first.model {
      ModelType::Standard(model) => model,
      ModelType::Foreign(_) => unreachable!(),
    };

    let model = model.read();
    let mesh = model.mesh.clone();
    let material = model.material.clone();

    let transforms = source
      .values()
      .map(|m| {
        let node = &m.read().node;
        let mat = d_sys.get_world_matrix(node);
        let net_visible = d_sys.get_net_visible(node);
        if net_visible {
          mat
        } else {
          Mat4::zero()
        }
      })
      .collect();

    let instance_mesh = TransformInstancedSceneMesh { mesh, transforms }.into_ref();

    let instance_model = StandardModel {
      material,
      mesh: SceneMeshType::TransformInstanced(instance_mesh),
      group: model.group,
      skeleton: None,
    }
    .into_ref();

    let instance_model = SceneModelImpl {
      model: ModelType::Standard(instance_model),
      node: new_nodes.create_node(Default::default()),
    }
    .into_ref();

    (instance_model, true).into()
  }
}
