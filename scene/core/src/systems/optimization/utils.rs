use futures::*;
use reactive::*;

use crate::*;

pub type OriginModelId = u64;
pub type ModelChange = ContainerRefRetainContentDelta<SceneModel>;
pub type ModelOutChange = ContainerRefRetainContentDelta<(SceneModel, bool)>;

pub fn model_transform<O>(
  scene_delta: impl Stream<Item = MixSceneDelta> + Unpin + 'static,
  transformer: impl FnOnce(
    Box<dyn Stream<Item = ModelChange> + Unpin>,
    &SceneNodeCollection,
    &SceneNodeDeriveSystem,
  ) -> O,
) -> (
  impl Stream<Item = MixSceneDelta>,
  TransformStat,
  Scene,
  SceneNodeDeriveSystem,
)
where
  O: Stream<Item = ModelOutChange>,
{
  let broad_cast = scene_delta.create_broad_caster();

  // split the model stream, maintain the old arena relationship
  let model_input = broad_cast
    .fork_stream()
    .filter_map_sync(|delta| match delta {
      MixSceneDelta::models(d) => Some(d),
      _ => None,
    });

  let (new_scene, new_derives) = SceneImpl::new();
  let output_derives = new_derives.clone();
  let middle_scene_nodes = new_scene.read().core.read().nodes.clone();
  let (model_input, original) = stat_model_count(model_input.batch_processing()); // todo improve batch
  let model_input = Box::new(model_input.flat_map(futures::stream::iter)); // this box is just to make the code easy to write

  let transformed_models =
    transformer(model_input, &middle_scene_nodes, &new_derives).map(|v| match v {
      ContainerRefRetainContentDelta::Remove((v, _)) => ContainerRefRetainContentDelta::Remove(v),
      ContainerRefRetainContentDelta::Insert((v, _)) => ContainerRefRetainContentDelta::Insert(v),
    });

  let (transformed_models, transformed) = stat_model_count(transformed_models.batch_processing());

  let transformed_models = transformed_models
    .flat_map(futures::stream::iter)
    .map(MixSceneDelta::models);

  // the other change stream
  let other_stuff = broad_cast
    .fork_stream()
    .filter_map_sync(|delta| match &delta {
      MixSceneDelta::models(_) => None,
      _ => Some(delta),
    });

  let output = futures::stream::select_with_strategy(other_stuff, transformed_models, prior_left);

  let stat = TransformStat {
    original,
    transformed,
  };

  (output, stat, new_scene, output_derives)
}

fn prior_left(_: &mut ()) -> stream::PollNext {
  stream::PollNext::Left
}

pub struct TransformStat {
  original: Arc<RwLock<u64>>,
  transformed: Arc<RwLock<u64>>,
}

impl TransformStat {
  pub fn original(&self) -> u64 {
    *self.original.read().unwrap()
  }
  pub fn transformed(&self) -> u64 {
    *self.transformed.read().unwrap()
  }
}

pub fn stat_model_count(
  input: impl Stream<Item = Vec<ModelChange>>,
) -> (impl Stream<Item = Vec<ModelChange>>, Arc<RwLock<u64>>) {
  let stat = Arc::new(RwLock::new(0_u64));
  let stat_c = stat.clone();
  let s = input.map(move |ms| {
    let mut stat = stat.write().unwrap();
    for m in &ms {
      match m {
        ContainerRefRetainContentDelta::Remove(_) => *stat -= 1,
        ContainerRefRetainContentDelta::Insert(_) => *stat += 1,
      }
    }
    ms
  });
  (s, stat_c)
}

pub trait RecyclableHashManyToOne: Clone {
  type Transformer: ModelProxy + Stream<Item = Vec<TransformerDelta>> + Unpin;
  type Key: Eq + Hash + Clone + Send + Sync;
  fn create_key(&self, model: &SceneModel) -> Self::Key;
  fn create_transformer(&self, key: Self::Key) -> Self::Transformer;
}

pub trait ModelProxy {
  fn insert_source_model(&mut self, model: SceneModel);
  fn remove_source_model_by_guid(&mut self, guid: u64);
}

/// we only care about the reference change here
/// the downstream could listen the new ref to get what they want.
pub enum TransformerDelta {
  ReleaseUnsuitable(SceneModel), // original model
  DropSource(SceneModel),        // original model
  NewTransformed(SceneModel, bool),
  RemoveTransformed(SceneModel, bool),
}

pub fn recyclable_hash_many_to_one<T: RecyclableHashManyToOne>(
  input: impl Stream<Item = ModelChange>,
  implementation: T,
) -> impl Stream<Item = ModelOutChange> {
  // origin model id => transformed id
  let mut source_id_transformer_map: FastHashMap<OriginModelId, T::Key> = Default::default();

  // transformed id => transformed
  let transformers: StreamMap<T::Key, T::Transformer> = StreamMap::default();

  let (recycling_sender, recycled_models) = futures::channel::mpsc::unbounded();

  let input = futures::stream::select_with_strategy(
    recycled_models.map(ModelChange::Insert),
    input,
    prior_left, // always drain recycled first, because message order matters.
  );

  input
    .fold_signal_state_stream(transformers, move |d, transformers| {
      match d {
        ModelChange::Insert(model) => {
          let idx = model.guid();
          // for any new coming model, calculate the mapping key, and find which existing
          // transformer could be merged with
          let key = implementation.create_key(&model);
          source_id_transformer_map.insert(idx, key.clone());

          // merge into the transformer or create a new transformer
          transformers
            .get_or_insert_with(key.clone(), || implementation.create_transformer(key))
            .insert_source_model(model);
        }
        ModelChange::Remove(model) => {
          let idx = model.guid();
          let key = source_id_transformer_map.remove(&idx).unwrap();

          let transformer = transformers.get_mut(&key).unwrap();
          // remove the source model from the inside of the transformer,
          // drop the source and eventually drop the transformer if no more source in it
          transformer.remove_source_model_by_guid(idx);
        }
      }
    })
    .flat_map(futures::stream::iter)
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
