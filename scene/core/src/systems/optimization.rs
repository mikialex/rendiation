use core::{
  pin::Pin,
  task::{Context, Poll},
};

use futures::*;
use reactive::{do_updates_by, once_forever_pending, SignalStreamExt, StreamMap, StreamMapDelta};
use rendiation_renderable_mesh::MeshDrawGroup;

use crate::*;

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
  // note, we have a subtle requirement that the other change in stream have no dependency over
  // model change in stream or we will have to handle it manually.
  pub fn new(
    scene_delta: impl Stream<Item = SceneInnerDelta> + Unpin,
    d_system: &SceneNodeDeriveSystem,
  ) -> (Self, impl Stream<Item = SceneInnerDelta>) {
    let broad_cast = scene_delta.create_broad_caster();

    // split the model stream, maintain the old arena relationship
    let model_input = broad_cast
      .fork_stream()
      .filter_map_sync(|delta| match delta {
        SceneInnerDelta::models(d) => Some(d),
        _ => None,
      })
      .map(IndependentItemContainerDelta::from)
      .transform_delta_to_ref_retained_by_hashing() // we could use single transformer
      .transform_ref_retained_to_ref_retained_content_by_hashing();

    let new_nodes = SceneNodeCollection::default();
    let new_nodes = Arc::new(new_nodes);

    let raw_tree_delta = broad_cast
      .fork_stream()
      .filter_map_sync(|delta| match delta {
        SceneInnerDelta::nodes(d) => Some(d),
        _ => None,
      });
    let new_node_changes = new_nodes.inner.visit_inner(|tree| tree.source.listen());
    let merged_tree_changes = merge_two_tree_deltas(raw_tree_delta, new_node_changes) //
      .map(SceneInnerDelta::nodes);

    // heavy logic in here!
    let transformed_models = instance_transform(model_input, d_system, &new_nodes)
      .transform_ref_retained_content_to_arena_by_hashing()
      .map(SceneInnerDelta::models);

    // the other change stream
    let other_stuff = broad_cast
      .fork_stream()
      .filter_map_sync(|delta| match &delta {
        SceneInnerDelta::models(_) | SceneInnerDelta::nodes(_) => None,
        _ => Some(delta),
      });

    let output = futures::stream::select(transformed_models, other_stuff);
    // drain new node change first to keep order valid
    let output = futures::stream::select_with_strategy(merged_tree_changes, output, prior_left);

    (Self {}, output)
  }
}

type OriginModelId = usize;
type ModelChange = ContainerRefRetainContentDelta<SceneModel>;

fn prior_left(_: &mut ()) -> stream::PollNext {
  stream::PollNext::Left
}

fn instance_transform(
  input: impl Stream<Item = ModelChange>,
  d_sys: &SceneNodeDeriveSystem,
  new_nodes: &Arc<SceneNodeCollection>,
) -> impl Stream<Item = ModelChange> {
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
          // for any new coming model , calculate instance key, find which exist instance could be
          // merged with
          let key = compute_instance_key(&model, &d_sys);
          source_id_transformer_map.insert(idx, key.clone());

          // merge into the transformer or create the transformer
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
          // remove the source model from the inside of transformer,
          // drop the source and eventually drop the transformer if no more source in it
          transformer.notify_source_dropped(idx);
        }
      }
    })
    .filter_map_sync(|delta| {
      match delta {
        StreamMapDelta::Delta(key, deltas) => (key, deltas).into(),
        _ => None, // we do not care the transformer add or delete here
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
            TransformerDelta::NewTransformed(transformed) => ModelChange::Insert(transformed),
            TransformerDelta::RemoveTransformed(transformed) => ModelChange::Remove(transformed),
          }
          .into()
        })
        .collect::<Vec<_>>(); // we have to do a collect because this iter borrows the recycling_sender

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

/// we call it transformer here because maybe this struct will likely be reused in other optimizer
#[pin_project::pin_project]
struct Transformer {
  d_sys: SceneNodeDeriveSystem,
  new_nodes: Arc<SceneNodeCollection>,
  key: PossibleInstanceKey,
  #[pin]
  source: StreamMap<usize, InstanceSourceStream>,
  source_model: HashMap<usize, SceneModel>,
  transformed: Option<SceneModel>,
}

impl Transformer {
  pub fn new(
    key: PossibleInstanceKey,
    d_sys: SceneNodeDeriveSystem,
    new_nodes: Arc<SceneNodeCollection>,
  ) -> Self {
    Self {
      key,
      d_sys,
      new_nodes,
      source: Default::default(),
      source_model: Default::default(),
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
  type Item = Vec<TransformerDelta>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    // we simple recreate new instance if any incremental source change (could optimize later)
    // so, here we do some batch process to avoid unnecessary instance rebuild
    let mut batched = Vec::<StreamMapDelta<usize, InstanceSourceIncrementalUpdate>>::new();
    do_updates_by(&mut this.source, cx, |d| batched.push(d));

    if batched.is_empty() && this.source_model.is_empty() {
      return Poll::Ready(None);
    }

    let mut results = Vec::new();
    let mut require_rebuild = false;
    let mut to_recycle = Vec::new();
    let mut to_remove = Vec::new();

    batched.drain(..).for_each(|d| match d {
      reactive::StreamMapDelta::Insert(_) => require_rebuild = true,
      reactive::StreamMapDelta::Remove(idx) => {
        to_remove.push(this.source_model.get(&idx).unwrap().clone());
        require_rebuild = true;
      }
      reactive::StreamMapDelta::Delta(idx, d) => match d {
        InstanceSourceIncrementalUpdate::WorldMat(_) => {
          // should optimize later
          require_rebuild = true;
        }
        InstanceSourceIncrementalUpdate::InstanceKeyChanged => {
          to_recycle.push(this.source_model.get(&idx).unwrap().clone());
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

    // recycle minus drop
    to_remove.iter().for_each(|m| {
      if let Some(to_recycle_but_dropped) =
        to_recycle.iter().position(|model| model.guid() == m.guid())
      {
        to_recycle.remove(to_recycle_but_dropped);
      }
    });

    use TransformerDelta::*;
    results.extend(to_remove.into_iter().map(DropSource));
    results.extend(to_recycle.into_iter().map(ReleaseUnsuitable));

    if require_rebuild {
      let new_transformed = create_instance(this.source_model, this.d_sys, this.new_nodes);
      results.push(TransformerDelta::NewTransformed(new_transformed.clone()));
      if let Some(old) = this.transformed.replace(new_transformed) {
        results.push(TransformerDelta::RemoveTransformed(old));
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
  new_nodes: &SceneNodeCollection,
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
      node: new_nodes.create_new_root(),
    }
    .into_ref()
  }
}
