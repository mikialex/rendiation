use std::{
  collections::HashSet,
  ops::DerefMut,
  sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::*;

use reactive::{EventDispatcher, Stream};

pub struct SceneItemRef<T: Incremental> {
  inner: Arc<RwLock<Identity<T>>>,
}

impl<T: Incremental + Send + Sync> SimpleIncremental for SceneItemRef<T> {
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    *self = delta;
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

impl<T: Incremental> Clone for SceneItemRef<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}
impl<T: Incremental> From<T> for SceneItemRef<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

pub struct Mutating<'a, T: Incremental> {
  pub inner: &'a mut T,
  pub collector: &'a mut dyn FnMut(&T, &T::Delta),
}

impl<'a, T: Incremental> Deref for Mutating<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.inner
  }
}

impl<'a, T: Incremental> Mutating<'a, T> {
  pub fn modify(&mut self, delta: T::Delta) {
    (self.collector)(self.inner, &delta);
    self.inner.apply(delta).unwrap()
  }

  pub fn trigger_manual(&mut self, delta: T::Delta) {
    (self.collector)(self.inner, &delta);
  }
}

impl<T: Incremental> SceneItemRef<T> {
  pub fn new(source: T) -> Self {
    let inner = Arc::new(RwLock::new(Identity::new(source)));
    Self { inner }
  }

  pub fn mutate<R>(&self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    let mut inner = self.inner.write().unwrap();
    let i: &mut Identity<T> = &mut inner;
    i.mutate(mutator)
  }
  pub fn visit<R>(&self, mut visitor: impl FnMut(&T) -> R) -> R {
    let inner = self.inner.read().unwrap();
    visitor(&inner)
  }

  pub fn read(&self) -> SceneItemRefGuard<T> {
    self
      .inner
      .read()
      .ok()
      .map(|inner| SceneItemRefGuard { inner })
      .unwrap()
  }
}

pub struct SceneItemRefGuard<'a, T: Incremental> {
  inner: RwLockReadGuard<'a, Identity<T>>,
}

impl<'a, T: Incremental> Deref for SceneItemRefGuard<'a, T> {
  type Target = Identity<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

pub struct SceneItemRefMutGuard<'a, T: Incremental> {
  inner: RwLockWriteGuard<'a, Identity<T>>,
}

impl<'a, T: Incremental> Deref for SceneItemRefMutGuard<'a, T> {
  type Target = Identity<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

impl<'a, T: Incremental> DerefMut for SceneItemRefMutGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.deref_mut()
  }
}

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Identity<T: Incremental> {
  id: usize,
  inner: T,
  change_dispatcher: EventDispatcher<DeltaView<'static, T>>,
  drop_dispatcher: EventDispatcher<()>,
}

impl<T: Incremental> AsRef<T> for Identity<T> {
  fn as_ref(&self) -> &T {
    &self.inner
  }
}

pub trait IntoSceneItemRef: Sized + Incremental {
  fn into_ref(self) -> SceneItemRef<Self> {
    self.into()
  }
}

impl<T: Incremental> IntoSceneItemRef for T {}

impl<T: Incremental> From<T> for Identity<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

impl<T: Incremental> Identity<T> {
  pub fn new(inner: T) -> Self {
    Self {
      inner,
      id: GLOBAL_ID.fetch_add(1, Ordering::Relaxed),
      change_dispatcher: Default::default(),
      drop_dispatcher: Default::default(),
    }
  }

  pub fn delta_stream(&self) -> Stream<DeltaView<'static, T>> {
    self.change_dispatcher.stream()
  }

  pub fn id(&self) -> usize {
    self.id
  }

  pub fn mutate<R>(&mut self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    let data = &mut self.inner;
    let dispatcher = &self.change_dispatcher;
    mutator(Mutating {
      inner: data,
      collector: &mut |data, delta| {
        let view = DeltaView { data, delta };
        let view = unsafe { std::mem::transmute(view) };
        dispatcher.emit(&view);
      },
    })
  }
}

impl<T: Default + Incremental> Default for Identity<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T: Incremental> Drop for Identity<T> {
  fn drop(&mut self) {
    self.drop_dispatcher.emit(&());
  }
}

impl<T: Incremental> std::ops::Deref for Identity<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

pub struct IdentityMapper<T, U: ?Sized> {
  data: HashMap<usize, T>,
  to_remove: Arc<RwLock<Vec<usize>>>,
  changed: Arc<RwLock<HashSet<usize>>>,
  phantom: PhantomData<U>,
}

impl<T, U: ?Sized> Default for IdentityMapper<T, U> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      to_remove: Default::default(),
      changed: Default::default(),
      phantom: Default::default(),
    }
  }
}

pub enum ResourceLogic<'a, 'b, T, U> {
  Create(&'a U),
  Update(&'b mut T, &'a U),
}
pub enum ResourceLogicResult<'a, T> {
  Create(T),
  Update(&'a mut T),
}

impl<'a, T> ResourceLogicResult<'a, T> {
  pub fn unwrap_new(self) -> T {
    match self {
      ResourceLogicResult::Create(v) => v,
      ResourceLogicResult::Update(_) => panic!(),
    }
  }

  pub fn unwrap_update(self) -> &'a mut T {
    match self {
      ResourceLogicResult::Create(_) => panic!(),
      ResourceLogicResult::Update(v) => v,
    }
  }
}

pub trait RequireMaintain: std::any::Any {
  fn maintain(&mut self);
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: 'static, U: 'static + ?Sized> RequireMaintain for IdentityMapper<T, U> {
  fn maintain(&mut self) {
    self.to_remove.write().unwrap().drain(..).for_each(|id| {
      self.data.remove(&id);
    });
  }
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}

impl<T: 'static, U: 'static + ?Sized> IdentityMapper<T, U> {
  /// this to bypass the borrow limits of get_update_or_insert_with
  pub fn get_update_or_insert_with_logic<'a, 'b, X: Incremental>(
    &'b mut self,
    source: &'a Identity<X>,
    mut logic: impl FnMut(ResourceLogic<'a, 'b, T, X>) -> ResourceLogicResult<'b, T>,
  ) -> &'b mut T {
    let mut new_created = false;
    let id = source.id;

    let mut resource = self.data.entry(id).or_insert_with(|| {
      let item = logic(ResourceLogic::Create(&source.inner)).unwrap_new();
      new_created = true;

      let weak_changed = Arc::downgrade(&self.changed);
      source.change_dispatcher.stream().on(move |_| {
        println!("update1");
        if let Some(change) = weak_changed.upgrade() {
          println!("update");
          change.write().unwrap().insert(id);
          false
        } else {
          true
        }
      });

      let weak_to_remove = Arc::downgrade(&self.to_remove);
      source.drop_dispatcher.stream().on(move |_| {
        if let Some(to_remove) = weak_to_remove.upgrade() {
          to_remove.write().unwrap().push(id);
          false
        } else {
          true
        }
      });

      item
    });

    if new_created || self.changed.write().unwrap().remove(&source.id) {
      resource = logic(ResourceLogic::Update(resource, source)).unwrap_update();
    }

    resource
  }

  pub fn get_update_or_insert_with<X: Incremental>(
    &mut self,
    source: &Identity<X>,
    mut creator: impl FnMut(&X) -> T,
    mut updater: impl FnMut(&mut T, &X),
  ) -> &mut T {
    self.get_update_or_insert_with_logic(source, |logic| match logic {
      ResourceLogic::Create(source) => ResourceLogicResult::Create(creator(source)),
      ResourceLogic::Update(mapped, source) => {
        updater(mapped, source);
        ResourceLogicResult::Update(mapped)
      }
    })
  }

  pub fn get_unwrap<X: Incremental>(&self, source: &Identity<X>) -> &T {
    self.data.get(&source.id).unwrap()
  }
}
