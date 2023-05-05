// use crate::*;

// use core::{
//   pin::Pin,
//   task::{Context, Poll},
// };
// use futures::*;
// use reactive::{IndexedItem, SignalStreamExt, StreamMap};

// // data flow:

// // standard + standard => instance
// // standard + instance => instance
// // instance + instance => instance

// // instance => standard
// // instance => instance + standard
// // instance => instance + instance (supported, but not directly)

// pub struct AutoInstanceSystem {
//   //
// }

// // input
// impl AutoInstanceSystem {
//   pub fn new(
//     scene_delta: impl Stream<Item = SceneInnerDelta>,
//     scene_derived_delta: impl Stream<Item = SceneNodeDerivedDataDelta>,
//   ) -> Self {
//     use arena::ArenaDelta::*;

//     // let instance_cache: HashMap<InstanceKey, InstanceProxy> = HashMap::new();

//     // scene_delta
//     //   .filter_map_sync(|delta| match delta {
//     //     SceneInnerDelta::models(delta) => match delta {
//     //       Mutate((model, idx)) => (idx.index(), build_instance_key_stream(model).into()),
//     //       Insert((model, idx)) => (idx.index(), build_instance_key_stream(model).into()),
//     //       Remove(idx) => (idx.index(), None),
//     //     }
//     //     .into(),
//     //     _ => None,
//     //   })
//     //   .flatten_into_vec_stream_signal()
//     //   .map(|d| match d {
//     //     reactive::VecUpdateUnit::Remove(_) => todo!(),
//     //     reactive::VecUpdateUnit::Active(_) => todo!(),
//     //     reactive::VecUpdateUnit::Update { index, item } => todo!(),
//     //   });

//     todo!()
//   }
// }

// // output
// impl Stream for AutoInstanceSystem {
//   type Item = SceneInnerDelta;

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
//     todo!()
//   }
// }

// #[derive(Hash, PartialEq, Eq)]
// struct InstanceKey {
//   pub material_id: usize,
//   pub mesh_id: usize,
//   pub world_mat_flip_side: bool,
// }

// #[derive(Hash, PartialEq, Eq)]
// enum PossibleInstanceKey {
//   UnableToInstance(usize), // just the origin model uuid
//   Instanced(InstanceKey),
// }

// #[derive(Incremental)]
// struct InstanceSource {
//   pub material: SceneMaterialType,
//   pub mesh_id: SceneMeshType,
//   pub world_mat: Mat4<f32>,
// }

// use std::hash::Hash;
// // world_mat_flip_side, ids
// impl Hash for InstanceSource {
//   fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//     todo!()
//   }
// }

// type InstanceSourceStream = impl Stream<Item = Option<InstanceSource>> + Unpin;
// fn build_instance_key_stream(model: &SceneModel) -> InstanceSourceStream {
//   // watch instance change, if the model type changed to unable to instance, return poll none
//   todo!()
// }

// struct Transformer {
//   key: PossibleInstanceKey,
//   source: StreamMap<(InstanceSourceStream, SceneModel)>,
//   /// if the source is single model, then the transformed model is the same source model
//   transformed: SceneModel,
// }

// impl Transformer {
//   pub fn add_new_source(&mut self, source: SceneModel) {
//     let model_pair = (build_instance_key_stream(&source), source);
//     self.source.insert(source.id(), model_pair);
//   }

//   pub fn notify_source_dropped(&mut self, source_id: usize) -> SceneModel {
//     self.source.remove(source_id).unwrap().1
//   }
// }

// /// we only care about the reference change here(create new transformed instance)
// /// the downstream could listen the new ref to get what they want.
// pub enum TransformerDelta {
//   ReleaseUnsuitable(SceneModel), // original model
//   NewTransformed(SceneModel),
//   RemoveTransformed(SceneModel),
// }

// impl Stream for Transformer {
//   type Item = TransformerDelta;

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//     // poll source, update change to transformed.

//     // if the source's key change, we emit the unsuitable source release

//     // if transformer is new created, we emit NewTransformed

//     // we always update reference for now, so we emit RemoveTransformed & NewTransformed
//     // the updating maybe result origin or transformed.

//     // if the source is empty, we return poll none
//     todo!()
//   }
// }

// type TransformedModelStream = impl Stream<Item = SceneModel> + Unpin;

// type OriginModelId = usize;
// type TransformedModelId = usize;

// enum ModelChange {
//   Insert(SceneModel),
//   Remove(usize),
// }

// fn create_instance_stream(
//   key: PossibleInstanceKey,
//   input: impl Stream<Item = ModelChange>,
// ) -> impl Stream<Item = ModelChange> {
//   let init = None;

//   // origin model id => transformed id
//   let mut source_id_transformer_map: HashMap<OriginModelId, TransformedModelId> = HashMap::new();

//   // current instance key => transformed id
//   let mut key_transformer_map: HashMap<PossibleInstanceKey, TransformedModelId> = HashMap::new();

//   // transformed id => transformed
//   let mut transformers: StreamMap<Transformer> = StreamMap::default();

//   let (recycling_sender, recycled_models) = futures::channel::mpsc::unbounded();

//   let input_handle = input.map(|d| {
//     match d {
//       ModelChange::Insert(model) => {
//         let idx = model.id();
//         // for any new coming model , calculate instance key, find which exist instance could be merged with
//         let key: PossibleInstanceKey = PossibleInstanceKey::UnableToInstance(idx);
//         if let Some(instance_idx) = key_transformer_map.get(&key) {
//           let transformer = transformers.get(*instance_idx).unwrap();
//           transformer.add_new_source(model);
//         } else {
//           // if we don't have any existing instance now, we create a new transformer
//           //   let new_id = instance_cache3.new_id();
//           //   instance_cache3.insert(Transformer);
//         }
//         let instance_idx = key_transformer_map.entry(key).or_insert_with(|| todo!());
//         source_id_transformer_map.insert(idx, *instance_idx);
//       }
//       ModelChange::Remove(idx) => {
//         // notify the transformer that one of it's source dropped
//         let transformed_id = source_id_transformer_map.remove(&idx).unwrap();
//         let transformer = transformers.get(transformed_id).unwrap();
//         transformer.notify_source_dropped(idx); // remove from the inside of transformer, drop the source
//       }
//     }
//   });

//   let transformed_handle = transformers.filter_map_sync(
//     |IndexedItem {
//        index: transform_model_id, // todo removal, and remove transformer, remove key
//        item: delta,
//      }| {
//       match delta {
//         TransformerDelta::ReleaseUnsuitable(source) => {
//           recycling_sender.unbounded_send(source);
//           return None;
//         }
//         TransformerDelta::NewTransformed(transformed) => ModelChange::Insert(transformed),
//         TransformerDelta::RemoveTransformed(source) => ModelChange::Remove(source.id()),
//       }
//       .into()
//     },
//   );

//   todo!();
// }
