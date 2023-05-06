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
    broad_cast
      .fork_stream()
      .filter_map_sync(|delta| match delta {
        SceneInnerDelta::models(delta) => match delta {
          Mutate((model, idx)) => {
            let previous = model_cache.remove(&idx.index()).unwrap();
            model_change_sender.unbounded_send(ModelChange::Remove(previous.id()));
            model_cache.insert(idx.index(), model.clone());
            model_change_sender.unbounded_send(ModelChange::Insert(model));
          }
          Insert((model, idx)) => {
            model_cache.insert(idx.index(), model.clone());
            model_change_sender.unbounded_send(ModelChange::Insert(model));
          }
          Remove(idx) => {
            let previous = model_cache.remove(&idx.index()).unwrap();
            model_change_sender.unbounded_send(ModelChange::Remove(previous.id()));
          }
        }
        .into(),
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
          output_remapping.insert(model.id(), handle);
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
        v => Some(delta),
      });

    let output = futures::stream::select(transformed_models, other_stuff);

    (Self {}, output)
  }
}

type OriginModelId = usize;
type TransformedModelId = usize;

enum ModelChange {
  Insert(SceneModel),
  Remove(usize),
}

fn instance_transform(
  input: impl Stream<Item = ModelChange>,
  d_sys: &SceneNodeDeriveSystem,
) -> impl Stream<Item = ModelChange> {
  // origin model id => transformed id
  let mut source_id_transformer_map: HashMap<OriginModelId, TransformedModelId> = HashMap::new();

  // current instance key => transformed id
  let mut key_transformer_map: HashMap<PossibleInstanceKey, TransformedModelId> = HashMap::new();

  // transformed id => transformed
  let mut transformers: StreamMap<Transformer> = StreamMap::default();

  let (recycling_sender, recycled_models) = futures::channel::mpsc::unbounded();

  let input = futures::stream::select(recycled_models.map(ModelChange::Insert), input);

  let input_handle = input.map(|d| {
    match d {
      ModelChange::Insert(model) => {
        let idx = model.id();
        // for any new coming model , calculate instance key, find which exist instance could be merged with
        let key = compute_instance_key(&model);
        if let Some(instance_idx) = key_transformer_map.get(&key) {
          let transformer = transformers.get(*instance_idx).unwrap();
          transformer.add_new_source(model, d_sys);
        } else {
          // if we don't have any existing instance now, we create a new transformer
          //   let new_id = instance_cache3.new_id();
          //   instance_cache3.insert(Transformer);
          todo!()
        }
        let instance_idx = key_transformer_map.entry(key).or_insert_with(|| todo!());
        source_id_transformer_map.insert(idx, *instance_idx);
      }
      ModelChange::Remove(idx) => {
        // notify the transformer that one of it's source dropped
        let transformed_id = source_id_transformer_map.remove(&idx).unwrap();
        let transformer = transformers.get(transformed_id).unwrap();
        transformer.notify_source_dropped(idx); // remove from the inside of transformer, drop the source
      }
    }
  });

  let transformed_handle =
    transformers.filter_map_sync(move |delta: StreamMapDelta<TransformerDelta>| {
      // match delta {
      //   TransformerDelta::ReleaseUnsuitable(source) => {
      //     recycling_sender.unbounded_send(source);
      //     return None;
      //   }
      //   TransformerDelta::NewTransformed(transformed) => ModelChange::Insert(transformed),
      //   TransformerDelta::RemoveTransformed(source) => ModelChange::Remove(source.id()),
      // }
      // .into()
      todo!()
    });

  transformed_handle.depend_pending_stream(input_handle)
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct InstanceKey {
  pub world_mat_flip_side: bool,
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

fn compute_instance_key(model: &SceneModel) -> PossibleInstanceKey {
  todo!()
}

fn compute_instance_key_inner(model: &SceneItemRef<StandardModel>) -> Option<InstanceContentKey> {
  todo!()
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

  let world_matrix = model
    .single_listen_by(with_field!(SceneModelImpl => node))
    .map(move |n| d.create_world_matrix_stream(&n))
    .flatten_signal()
    .map(InstanceSourceIncrementalUpdate::WorldMat);

  let model = model
    .single_listen_by(with_field!(SceneModelImpl => model))
    .map(move |model| match model {
      ModelType::Standard(sm) => {
        let model_ref = sm.downgrade();
        let key = key.clone();
        let watch = sm.unbound_listen_by(all_delta).filter_map_sync(move |d| {
          if let Some(model_ref) = model_ref.upgrade() {
            let new_key = compute_instance_key_inner(&model_ref);
            match (new_key, &key) {
              (None, PossibleInstanceKey::UnableToInstance(_)) => return None,
              (Some(new_key), PossibleInstanceKey::Instanced(key)) => {
                if new_key != key.content {
                  return InstanceSourceIncrementalUpdate::InstanceKeyChanged.into();
                } else {
                  return None;
                }
              }
              _ => return InstanceSourceIncrementalUpdate::InstanceKeyChanged.into(),
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

/// we call it transformer here because maybe this struct will be reused in other optimizer
#[pin_project::pin_project]
struct Transformer {
  key: PossibleInstanceKey,
  #[pin]
  source: StreamMap<InstanceSourceStream>,
  source_model: HashMap<usize, SceneModel>,
  state: CurrentTransformedState,
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
  pub fn new(key: PossibleInstanceKey) -> Self {
    Self {
      key,
      source: Default::default(),
      source_model: Default::default(),
      state: Default::default(),
      recycle_queue: Default::default(),
      require_rebuild: true,
    }
  }

  fn add_new_source(&mut self, source: SceneModel, d: &SceneNodeDeriveSystem) {
    let change = build_instance_source_stream(&source, d, self.key.clone());
    self.source.insert(source.id(), change);
    self.source_model.insert(source.id(), source);
  }

  fn notify_source_dropped(&mut self, source_id: usize) {
    self.source.remove(source_id).unwrap();
    // note, we not remove the source_model map here, the stream polling will do this
  }
}

/// we only care about the reference change here(create new transformed instance)
/// the downstream could listen the new ref to get what they want.
pub enum TransformerDelta {
  ReleaseUnsuitable(SceneModel), // original model
  NewTransformed(SceneModel),
  RemoveTransformed(SceneModel),
}

impl Stream for Transformer {
  type Item = TransformerDelta;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    if let CurrentTransformedState::Staging(model) = this.state {
      *this.state = CurrentTransformedState::Present(model.clone());
      return Poll::Ready(TransformerDelta::NewTransformed(model.clone()).into());
    }

    let mut batched = Vec::<StreamMapDelta<InstanceSourceIncrementalUpdate>>::new();
    do_updates_by(&mut this.source, cx, |d| batched.push(d));

    // we simple recreate new instance if any incremental source change (could optimize later)
    // so, here we do some batch process to avoid unnecessary instance rebuild
    batched.drain(..).for_each(|d| match d {
      reactive::StreamMapDelta::Insert(_) => *this.require_rebuild = true,
      reactive::StreamMapDelta::Remove(idx) => {
        let _ = this.source_model.remove(&idx).unwrap();
        // note: here we do not push the removed into recycle queue, because the source is removed or dropped
      }
      reactive::StreamMapDelta::Delta(idx, d) => match d {
        InstanceSourceIncrementalUpdate::WorldMat(_) => {
          *this.require_rebuild = false;
        }
        InstanceSourceIncrementalUpdate::InstanceKeyChanged => {
          let model = this.source_model.remove(&idx).unwrap();
          this.recycle_queue.push(model)
        }
      },
    });

    if let Some(r) = this.recycle_queue.pop() {
      return Poll::Ready(TransformerDelta::ReleaseUnsuitable(r).into());
    }

    if *this.require_rebuild {
      *this.require_rebuild = false;

      let new_transformed = create_instance(this.source_model);
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

    // if the source is empty, we return poll none
    if this.source_model.is_empty() {
      return Poll::Ready(None);
    }

    Poll::Pending
  }
}

/// if the source is single model, then the transformed model is the same source model
fn create_instance(source: &HashMap<usize, SceneModel>) -> SceneModel {
  if source.len() == 1 {
    source.values().next().unwrap().clone()
  } else {
    todo!()
  }
}
