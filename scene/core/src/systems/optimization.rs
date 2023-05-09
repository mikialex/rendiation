use crate::*;

use arena::{Arena, ArenaDelta, Handle};
use core::{
  pin::Pin,
  task::{Context, Poll},
};
use futures::*;
use reactive::{do_updates_by, once_forever_pending, SignalStreamExt, StreamMap, StreamMapDelta};
use rendiation_renderable_mesh::MeshDrawGroup;

// data flow:

// standard + standard => instance
// standard + instance => instance
// instance + instance => instance (not supported yet)

// instance => standard
// instance => instance + standard
// instance => instance + instance (supported indirectly)

pub struct AutoInstanceSystem {
  // maybe add some metrics collecting logic here?
}

// input
impl AutoInstanceSystem {
  // note, we have a subtle requirement that the other change in stream have no dependency over model change in stream
  // or we will have to handle it manually.
  pub fn new(
    scene_delta: impl Stream<Item = SceneInnerDelta> + Unpin,
    d_system: &SceneNodeDeriveSystem,
  ) -> (Self, impl Stream<Item = SceneInnerDelta>) {
    use arena::ArenaDelta::*;

    let broad_cast = scene_delta.create_broad_caster();

    let mut model_cache: HashMap<usize, SceneModel> = HashMap::new();

    let (model_change_sender, models_to_transform) = futures::channel::mpsc::unbounded();

    // split the model stream, maintain the old arena relationship
    let model_input = broad_cast
      .fork_stream()
      .filter_map_sync(move |delta| match delta {
        SceneInnerDelta::models(delta) => {
          match delta {
            Mutate((model, idx)) => {
              let previous = model_cache.remove(&idx.index()).unwrap();
              model_change_sender
                .unbounded_send(ModelChange::Remove(previous.guid()))
                .ok()?;
              model_cache.insert(idx.index(), model.clone());
              model_change_sender
                .unbounded_send(ModelChange::Insert(model))
                .ok()?;
            }
            Insert((model, idx)) => {
              model_cache.insert(idx.index(), model.clone());
              model_change_sender
                .unbounded_send(ModelChange::Insert(model))
                .ok()?;
            }
            Remove(idx) => {
              let previous = model_cache.remove(&idx.index()).unwrap();
              model_change_sender
                .unbounded_send(ModelChange::Remove(previous.guid()))
                .ok()?;
            }
          };
          ().into()
        }
        _ => None,
      });

    // heavy logic in here
    let transformed_models = instance_transform(models_to_transform, d_system);

    let mut output_arena = Arena::new();
    let mut output_remapping: HashMap<usize, Handle<SceneModel>> = Default::default();
    let transformed_models = transformed_models
      .map(move |model| match model {
        ModelChange::Insert(model) => {
          let handle = output_arena.insert(model.clone());
          output_remapping.insert(model.guid(), handle);
          ArenaDelta::Insert((model, handle))
        }
        ModelChange::Remove(index) => {
          let handle = output_remapping.remove(&index).unwrap();
          output_arena.remove(handle).unwrap();
          ArenaDelta::Remove(handle)
        }
      })
      .map(SceneInnerDelta::models);

    // split the other stream
    let other_stuff = broad_cast
      .fork_stream()
      .filter_map_sync(|delta| match &delta {
        SceneInnerDelta::models(_) => None,
        _ => Some(delta),
      });

    let output = futures::stream::select(transformed_models, other_stuff);

    let output = model_input.after_pended_then(output);

    (Self {}, output)
  }
}

type OriginModelId = usize;

enum ModelChange {
  Insert(SceneModel),
  Remove(usize),
}

fn instance_transform(
  input: impl Stream<Item = ModelChange>,
  d_sys: &SceneNodeDeriveSystem,
) -> impl Stream<Item = ModelChange> {
  // origin model id => transformed id
  let mut source_id_transformer_map: HashMap<OriginModelId, PossibleInstanceKey> = HashMap::new();

  // transformed id => transformed
  let transformers: StreamMap<PossibleInstanceKey, Transformer> = StreamMap::default();

  let (recycling_sender, recycled_models) = futures::channel::mpsc::unbounded();

  fn prio_left(_: &mut ()) -> stream::PollNext {
    stream::PollNext::Left
  }
  let input = futures::stream::select_with_strategy(
    recycled_models.map(ModelChange::Insert),
    input,
    prio_left, // always drain recycled first, because message order matters.
  );

  let d_sys = d_sys.clone();
  input
    .fold_signal_state_stream(transformers, move |d, transformers| {
      match d {
        ModelChange::Insert(model) => {
          let idx = model.guid();
          // for any new coming model , calculate instance key, find which exist instance could be merged with
          let key = compute_instance_key(&model, &d_sys);

          // merge into the transformer or create the transformer
          if let Some(transformer) = transformers.get_mut(&key) {
            transformer.add_new_source(model, &d_sys);
          } else {
            let transformer = Transformer::new(key.clone(), d_sys.clone());
            transformers.insert(key.clone(), transformer);
          }

          source_id_transformer_map.insert(idx, key.clone());
        }
        ModelChange::Remove(idx) => {
          let key = source_id_transformer_map.remove(&idx).unwrap();

          let transformer = transformers.get_mut(&key).unwrap();
          // remove the source model from the inside of transformer,
          // drop the source and eventually drop the transformer if no more source in it
          transformer.notify_source_dropped(idx);
        }
      }
    })
    .batch_processing() // note: we have to batch processing here to prevent put deleted model into the recycle queue.
    .map(move |deltas| {
      let mut to_recycle = smallvec::SmallVec::<[SceneModel; 3]>::default();
      let transform_change = deltas
        .into_iter()
        .filter_map(|delta| {
          match delta {
            StreamMapDelta::Insert(_) => return None,
            StreamMapDelta::Remove(_) => return None,
            StreamMapDelta::Delta(_, delta) => match delta {
              TransformerDelta::ReleaseUnsuitable(source) => {
                to_recycle.push(source);
                return None;
              }
              TransformerDelta::DropSource(source) => {
                if let Some(should_not_to_recycle) =
                  to_recycle.iter().position(|m| m.guid() == source.guid())
                {
                  to_recycle.remove(should_not_to_recycle);
                }
                return None;
              }
              TransformerDelta::NewTransformed(transformed) => ModelChange::Insert(transformed),
              TransformerDelta::RemoveTransformed(source) => ModelChange::Remove(source.guid()),
            },
          }
          .into()
        })
        .collect::<Vec<_>>();

      to_recycle.into_iter().for_each(|m| {
        recycling_sender.unbounded_send(m).ok();
      });

      futures::stream::iter(transform_change)
    })
    .flatten()
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct InstanceKey {
  pub is_front_side: bool,
  pub content: InstanceContentKey,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct InstanceContentKey {
  pub material_id: usize,
  pub mesh_id: usize,
  pub group: MeshDrawGroup,
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum PossibleInstanceKey {
  UnableToInstance(usize), // just the origin model uuid
  Instanced(InstanceKey),
}

enum InstanceSourceIncrementalUpdate {
  WorldMat(Mat4<f32>),
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

fn compute_instance_key_inner(model: &SceneItemRef<StandardModel>) -> Option<InstanceContentKey> {
  let model = model.read();
  InstanceContentKey {
    material_id: model.material.guid()?,
    mesh_id: model.mesh.guid()?,
    group: model.group,
  }
  .into()
}

/// we call it transformer here because maybe this struct will be reused in other optimizer
#[pin_project::pin_project]
struct Transformer {
  d_sys: SceneNodeDeriveSystem,
  key: PossibleInstanceKey,
  #[pin]
  source: StreamMap<usize, InstanceSourceStream>,
  source_model: HashMap<usize, SceneModel>,
  state: CurrentTransformedState,
  source_drop_queue: Vec<SceneModel>,
  recycle_queue: Vec<SceneModel>,
  require_rebuild: bool,
}

#[derive(Default, Clone)]
enum CurrentTransformedState {
  Staging(SceneModel),
  Present(SceneModel),
  #[default]
  NotInit,
}

impl Transformer {
  pub fn new(key: PossibleInstanceKey, d_sys: SceneNodeDeriveSystem) -> Self {
    Self {
      key,
      d_sys,
      source: Default::default(),
      source_model: Default::default(),
      state: Default::default(),
      source_drop_queue: Default::default(),
      recycle_queue: Default::default(),
      require_rebuild: true,
    }
  }

  fn add_new_source(&mut self, source: SceneModel, d: &SceneNodeDeriveSystem) {
    let change = build_instance_source_stream(&source, d, self.key.clone());
    self.source.insert(source.guid(), change);
    self.source_model.insert(source.guid(), source);
  }

  fn notify_source_dropped(&mut self, source_id: usize) {
    let _ = self.source.remove(source_id).unwrap();
    // note, we not remove the source_model map here, the stream polling will do this automatically
  }
}

/// we only care about the reference change here(create new transformed instance)
/// the downstream could listen the new ref to get what they want.
pub enum TransformerDelta {
  ReleaseUnsuitable(SceneModel), // original model
  DropSource(SceneModel),        // original model
  NewTransformed(SceneModel),
  RemoveTransformed(SceneModel),
}

impl Stream for Transformer {
  type Item = TransformerDelta;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    if let CurrentTransformedState::Staging(model) = this.state {
      let model = model.clone();
      *this.state = CurrentTransformedState::Present(model.clone());
      return Poll::Ready(TransformerDelta::NewTransformed(model).into());
    }

    // we simple recreate new instance if any incremental source change (could optimize later)
    // so, here we do some batch process to avoid unnecessary instance rebuild
    let mut batched = Vec::<StreamMapDelta<usize, InstanceSourceIncrementalUpdate>>::new();
    do_updates_by(&mut this.source, cx, |d| batched.push(d));

    batched.drain(..).for_each(|d| match d {
      reactive::StreamMapDelta::Insert(_) => *this.require_rebuild = true,
      reactive::StreamMapDelta::Remove(idx) => {
        // note:  we not unwrap to asset because the recycle side also do removal
        // note: here we do not push the removed into recycle queue, because the source is removed or dropped
        let _ = this.source_model.remove(&idx);
        *this.require_rebuild = true;
      }
      reactive::StreamMapDelta::Delta(idx, d) => match d {
        InstanceSourceIncrementalUpdate::WorldMat(_) => {
          *this.require_rebuild = true;
        }
        InstanceSourceIncrementalUpdate::InstanceKeyChanged => {
          // we did not filter out the key change and removal in history, so maybe removed in previous batched message,
          if let Some(model) = this.source_model.remove(&idx) {
            this.recycle_queue.push(model)
          }
        }
      },
    });

    if let Some(r) = this.recycle_queue.pop() {
      return Poll::Ready(TransformerDelta::DropSource(r).into());
    }

    if let Some(r) = this.recycle_queue.pop() {
      return Poll::Ready(TransformerDelta::ReleaseUnsuitable(r).into());
    }

    // if the source is empty, we return poll none
    if this.source_model.is_empty() {
      return Poll::Ready(None);
    }

    if *this.require_rebuild {
      *this.require_rebuild = false;

      let new_transformed = create_instance(this.source_model, this.d_sys);
      let re = match this.state.clone() {
        CurrentTransformedState::Present(old) => {
          *this.state = CurrentTransformedState::Staging(new_transformed);
          TransformerDelta::RemoveTransformed(old)
        }
        CurrentTransformedState::NotInit => {
          *this.state = CurrentTransformedState::Present(new_transformed.clone());
          TransformerDelta::NewTransformed(new_transformed)
        }
        CurrentTransformedState::Staging(_) => unreachable!(), // we early returned before
      };
      return Poll::Ready(re.into());
    }

    Poll::Pending
  }
}

type InstanceSourceStream = impl Stream<Item = InstanceSourceIncrementalUpdate> + Unpin;
type BoxedWatcher = Box<dyn Stream<Item = InstanceSourceIncrementalUpdate> + Unpin>;

// watch a model to check if the model's instance key matches the key passed in
// and return the stream of InstanceSourceIncrementalUpdate
fn build_instance_source_stream(
  model: &SceneModel,
  d: &SceneNodeDeriveSystem,
  key: PossibleInstanceKey,
) -> InstanceSourceStream {
  let d = d.clone();

  let is_font_side = if let PossibleInstanceKey::Instanced(key) = &key {
    key.is_front_side.into()
  } else {
    None
  };

  let world_matrix = model
    .single_listen_by(with_field!(SceneModelImpl => node))
    .map(move |n| d.create_world_matrix_stream(&n))
    .flatten_signal()
    .filter_map_sync(move |mat| {
      is_font_side.map(|is_font_side| {
        if is_front_side(mat) == is_font_side {
          InstanceSourceIncrementalUpdate::WorldMat(mat)
        } else {
          InstanceSourceIncrementalUpdate::InstanceKeyChanged
        }
      })
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
              (None, PossibleInstanceKey::UnableToInstance(_)) => None,
              (Some(new_key), PossibleInstanceKey::Instanced(key)) => {
                if new_key != key.content {
                  InstanceSourceIncrementalUpdate::InstanceKeyChanged.into()
                } else {
                  None
                }
              }
              _ => InstanceSourceIncrementalUpdate::InstanceKeyChanged.into(),
            }
          } else {
            None
          }
        });

        Box::new(watch) as BoxedWatcher
      }
      ModelType::Foreign(_) => match key {
        PossibleInstanceKey::UnableToInstance(_) => {
          Box::new(futures::stream::pending()) as BoxedWatcher
        }
        PossibleInstanceKey::Instanced(_) => Box::new(once_forever_pending(
          InstanceSourceIncrementalUpdate::InstanceKeyChanged,
        )),
      },
    })
    .flatten_signal();

  futures::stream::select(world_matrix, model)
}

fn create_instance(
  source: &HashMap<usize, SceneModel>,
  d_sys: &SceneNodeDeriveSystem,
) -> SceneModel {
  // if the source is single model, then the transformed model is the same source model
  if source.len() == 1 {
    source.values().next().unwrap().clone()
  } else {
    let first = source.values().next().unwrap();
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
      .map(|m| d_sys.get_world_matrix(&m.read().node))
      .collect();

    let instance_mesh = TransformInstancedSceneMesh { mesh, transforms }.into_ref();

    let instance_model = StandardModel {
      material,
      mesh: SceneMeshType::TransformInstanced(instance_mesh),
      group: model.group,
      skeleton: None,
    }
    .into_ref();

    SceneModelImpl {
      model: ModelType::Standard(instance_model),
      node: todo!(),
    }
    .into_ref()
  }
}
