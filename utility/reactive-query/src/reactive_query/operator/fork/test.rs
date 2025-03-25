use reactive_stream::noop_ctx;

use crate::*;

#[derive(Clone)]
pub struct TestReactiveMap<K, V> {
  inner: Arc<RwLock<FastHashMap<K, V>>>,
  sender: CollectiveMutationSender<K, V>,
}

impl<K: CKey, V: CValue> TestReactiveMap<K, V> {
  pub fn insert(&self, k: K, v: V) {
    let p = self.inner.write().insert(k.clone(), v.clone());
    unsafe {
      self.sender.lock();
      self.sender.send(k, ValueChange::Delta(v, p));
      self.sender.unlock();
    }
  }

  pub fn remove(&self, k: K) {
    let p = self.inner.write().remove(&k);
    if let Some(p) = p {
      unsafe {
        self.sender.lock();
        self.sender.send(k, ValueChange::Remove(p));
        self.sender.unlock();
      }
    }
  }
}

impl<K: CKey, V: CValue> QueryProvider<K, V> for TestReactiveMap<K, V> {
  fn access(&self) -> BoxedDynQuery<K, V> {
    self.inner.make_read_holder().into_boxed()
  }
}

pub fn create_test_map<K: CKey, V: CValue>() -> (
  TestReactiveMap<K, V>,
  impl ReactiveQuery<Key = K, Value = V>,
) {
  let (sender, rev) = collective_channel();

  let map = TestReactiveMap {
    inner: Default::default(),
    sender,
  };

  let query = ReactiveQueryFromCollectiveMutation {
    full: Box::new(map.clone()),
    mutation: RwLock::new(rev),
  };

  (map, query)
}

fn assert_vec_content_equal<T: std::hash::Hash + Eq + std::fmt::Debug>(a: &[T], b: &[T]) {
  assert_eq!(a.len(), b.len());
  let a = a.iter().collect::<FastHashSet<_>>();
  let b = b.iter().collect::<FastHashSet<_>>();
  assert_eq!(a, b);
}

#[test]
fn test_fork_basic() {
  let (map, query) = create_test_map::<u32, u32>();

  let a = query.into_forker();
  let b = a.clone().debug("a", false);

  map.insert(1, 1);
  map.insert(2, 1);
  map.remove(2);

  noop_ctx!(cx);
  // test basic function
  {
    let (a_d, a_v) = a.describe(cx).resolve();
    let (b_d, b_v) = b.describe(cx).resolve();

    let delta = vec![(1, ValueChange::Delta(1, None))];

    let a_delta = a_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&a_delta, &delta);

    let b_delta = b_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&b_delta, &delta);

    let view = vec![(1, 1)];

    let a_view = a_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&a_view, &view);

    let b_view = b_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&b_view, &view);
  }

  // later forked query should has init messages
  {
    let c = a.clone().debug("c", false);
    let (c_d, c_v) = c.describe(cx).resolve();
    let delta = vec![(1, ValueChange::Delta(1, None))];
    let c_delta = c_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_delta, &delta);

    let view = vec![(1, 1)];
    let c_view = c_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_view, &view);

    let (c_d, c_v) = c.describe(cx).resolve();
    assert_eq!(c_d.iter_key_value().count(), 0);
    assert_eq!(c_v.iter_key_value().count(), 1);

    drop(c);
    // raii should clean up downstream
    assert_eq!(a.downstream_count(), 2);
  }

  // once we polled change, change should be empty
  {
    let (a_d, a_v) = a.describe(cx).resolve();
    assert_eq!(a_d.iter_key_value().count(), 0);
    assert_eq!(a_v.iter_key_value().count(), 1);

    let (b_d, b_v) = b.describe(cx).resolve();
    assert_eq!(b_d.iter_key_value().count(), 0);
    assert_eq!(b_v.iter_key_value().count(), 1);
  }

  map.insert(2, 1);

  // described compute can coexist with each other
  // and also test new message can be received
  {
    let mut a_des = a.describe(cx);
    let mut b_des = b.describe(cx);
    let (a_d, a_v) = a_des.resolve();
    let (b_d, b_v) = b_des.resolve();

    let delta = vec![(2, ValueChange::Delta(1, None))];

    let a_delta = a_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&a_delta, &delta);

    let b_delta = b_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&b_delta, &delta);

    let view = vec![(1, 1), (2, 1)];

    let a_view = a_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&a_view, &view);

    let b_view = b_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&b_view, &view);
  }

  map.insert(3, 1);

  let c = a.clone();

  // polled result can coexist with described computes
  // and also test new message can be received
  {
    let (a_d, a_v) = a.describe(cx).resolve();
    let mut b_des = b.describe(cx);
    let (b_d, b_v) = b_des.resolve();
    let mut c_des = c.describe(cx);
    let (c_d, c_v) = c_des.resolve();

    let delta = vec![(3, ValueChange::Delta(1, None))];

    let a_delta = a_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&a_delta, &delta);

    let b_delta = b_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&b_delta, &delta);

    let c_delta = c_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(
      &c_delta,
      &[
        (1, ValueChange::Delta(1, None)),
        (2, ValueChange::Delta(1, None)),
        (3, ValueChange::Delta(1, None)),
      ],
    );

    let view = vec![(1, 1), (2, 1), (3, 1)];

    let a_view = a_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&a_view, &view);

    let b_view = b_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&b_view, &view);

    let c_view = c_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_view, &view);
  }

  map.insert(4, 1);
  // resolve order must match describe order.
  {
    let mut a_des = a.describe(cx);
    let mut a_des2 = a.describe(cx);
    let mut b_des = b.describe(cx);

    let (a_d, _a_v) = a_des.resolve();
    assert_eq!(a_d.iter_key_value().count(), 1);

    let (a_d, _a_v) = a_des2.resolve();
    assert_eq!(a_d.iter_key_value().count(), 0);

    let (b_d, _b_v) = b_des.resolve();
    assert_eq!(b_d.iter_key_value().count(), 1);
  }
}

#[test]
fn test_fork_diamond() {
  let (map, query) = create_test_map::<u32, u32>();

  let a = query.into_forker();
  let b = a.clone();

  let c = a.collective_zip(b);
  let c = c.into_forker();

  let d = c.clone();

  map.insert(1, 1);

  noop_ctx!(cx);

  {
    let mut c_des = c.describe(cx);
    let (d_d, d_v) = d.describe(cx).resolve();
    let (c_d, c_v) = c_des.resolve();

    let delta = vec![(1, ValueChange::Delta((1, 1), None))];
    let c_delta = c_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_delta, &delta);
    let d_delta = d_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&d_delta, &delta);

    let view = vec![(1, (1, 1))];
    let c_view = c_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_view, &view);
    let d_view = d_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&d_view, &view);
  };
  {
    let mut c_des = c.describe(cx);
    let (d_d, d_v) = d.describe(cx).resolve();
    let (c_d, c_v) = c_des.resolve();

    let delta = vec![];
    let c_delta = c_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_delta, &delta);
    let d_delta = d_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&d_delta, &delta);

    let view = vec![(1, (1, 1))];
    let c_view = c_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_view, &view);
    let d_view = d_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&d_view, &view);
  };
}

#[test]
fn test_fork_diamond2() {
  let (map1, query1) = create_test_map::<u32, u32>();
  let a = query1.into_forker();
  let (map2, query2) = create_test_map::<u32, u32>();
  let b = query2.into_forker();

  let c = a.collective_zip(b).collective_execute_map_by(|| |_, v| v);
  let c = c.into_forker();

  let d = c.clone();

  map1.insert(1, 1);
  map2.insert(1, 1);

  noop_ctx!(cx);

  {
    let mut c_des = c.describe(cx);
    let (c_d, c_v) = c_des.resolve();

    let delta = vec![(1, ValueChange::Delta((1, 1), None))];
    let c_delta = c_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_delta, &delta);
    let view = vec![(1, (1, 1))];
    let c_view = c_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_view, &view);

    let (d_d, d_v) = d.describe(cx).resolve();
    let d_delta = d_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&d_delta, &delta);
    let d_view = d_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&d_view, &view);
  };

  {
    let mut c_des = c.describe(cx);
    let (c_d, c_v) = c_des.resolve();

    let delta = vec![];
    let c_delta = c_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_delta, &delta);
    let view = vec![(1, (1, 1))];
    let c_view = c_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&c_view, &view);

    let (d_d, d_v) = d.describe(cx).resolve();
    let d_delta = d_d.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&d_delta, &delta);
    let d_view = d_v.iter_key_value().collect::<Vec<_>>();
    assert_vec_content_equal(&d_view, &view);
  };
}
