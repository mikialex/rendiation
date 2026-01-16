use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use fast_hash_collection::FastHashMap;
use ordered_float::OrderedFloat;
use rendiation_scene_rendering_gpu_base::SceneModelRenderBatch;

pub trait RenderBatchCollector {
  fn collect_batch(&mut self, batch: SceneModelRenderBatch);
  fn flush_frame(&mut self);
}

pub struct DoNothingRenderBatchCollector;

impl RenderBatchCollector for DoNothingRenderBatchCollector {
  fn collect_batch(&mut self, _batch: SceneModelRenderBatch) {}
  fn flush_frame(&mut self) {}
}

// todo, consider move this to another crate
#[derive(Default)]
pub struct BasicScheduler {
  weights: WeightOrdered<u32, OrderedFloat<f32>>,
  feedback_query: Option<Pin<Box<dyn Future<Output = ScheduleItemUseCountFeedBack>>>>,
}

pub struct ScheduleItemUseCountFeedBack {
  /// (item_id, weight)
  pub results: Vec<(u32, f32)>,
}

impl BasicScheduler {
  pub fn setup_new_feedback(
    &mut self,
    setup: impl FnOnce() -> Pin<Box<dyn Future<Output = ScheduleItemUseCountFeedBack>>>,
  ) {
    if self.feedback_query.is_none() {
      self.feedback_query = Some(setup());
    }
  }

  pub fn poll_feedback(&mut self, cx: &mut Context) {
    if let Some(fut) = &mut self.feedback_query {
      if let Poll::Ready(r) = Pin::new(fut).poll(cx) {
        // todo, ordered weights update is costly, one thing we can do is to split the feedback write into multiple
        // frames to avoid blocking in one frame
        let mut has_nan = false;
        for (id, new_weight) in r.results {
          if new_weight.is_nan() {
            has_nan = true;
          }
          let new_weight = OrderedFloat(new_weight);
          self.weights.update_or_insert(id, new_weight);
        }
        self.feedback_query = None;
        if has_nan {
          log::warn!("schedule feedback has nan");
        }
      }
    }
  }

  pub fn iter_weights(&mut self) -> impl Iterator<Item = &u32> {
    self.weights.iter_from_largest_weight()
  }
}

#[allow(clippy::disallowed_types)]
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
struct WeightedKey<K, Weight> {
  key: K,
  weight: Weight,
}

struct WeightOrdered<K, Weight> {
  #[allow(clippy::disallowed_types)]
  tree: BTreeSet<WeightedKey<K, Weight>>,
  weight_map: FastHashMap<K, Weight>,
}

impl<K, Weight> Default for WeightOrdered<K, Weight> {
  fn default() -> Self {
    Self {
      tree: Default::default(),
      weight_map: Default::default(),
    }
  }
}

impl<K, Weight> WeightOrdered<K, Weight>
where
  K: Eq + std::hash::Hash + Clone + Ord + Eq,
  Weight: Clone + Ord + Eq,
{
  fn update_or_insert(&mut self, key: K, weight: Weight) {
    self.remove(&key);
    self.weight_map.insert(key.clone(), weight.clone());
    self.tree.insert(WeightedKey { key, weight });
  }

  fn remove(&mut self, key: &K) {
    if let Some(weight) = self.weight_map.remove(key) {
      self.tree.remove(&WeightedKey {
        key: key.clone(),
        weight,
      });
    }
  }

  fn iter_from_largest_weight(&self) -> impl Iterator<Item = &K> {
    self.tree.iter().rev().map(|x| &x.key)
  }
}
