use std::{
  collections::{HashMap, HashSet},
  marker::PhantomData,
  sync::{Arc, RwLock},
};

use incremental::Incremental;

use super::identity::Identity;

pub struct IdentityMapper<T, U: ?Sized> {
  data: HashMap<usize, (T, bool)>,
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

impl<T: 'static, U: 'static + ?Sized> IdentityMapper<T, U> {
  pub fn check_clean_up(&mut self) {
    self.to_remove.write().unwrap().drain(..).for_each(|id| {
      self.data.remove(&id);
    });
    self.changed.write().unwrap().drain().for_each(|id| {
      self.data.get_mut(&id).unwrap().1 = true;
    })
  }

  /// this to bypass the borrow limits of get_update_or_insert_with
  pub fn get_update_or_insert_with_logic<'a, 'b, X: Incremental>(
    &'b mut self,
    source: &'a Identity<X>,
    mut logic: impl FnMut(ResourceLogic<'a, 'b, T, X>) -> ResourceLogicResult<'b, T>,
  ) -> &'b mut T {
    self.check_clean_up();

    let mut new_created = false;
    let id = source.id;

    let (resource, is_dirty) = self.data.entry(id).or_insert_with(|| {
      let item = logic(ResourceLogic::Create(&source.inner)).unwrap_new();
      new_created = true;

      let weak_changed = Arc::downgrade(&self.changed);
      source.change_dispatcher.stream().on(move |_| {
        if let Some(change) = weak_changed.upgrade() {
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

      (item, false)
    });

    if new_created || *is_dirty {
      *is_dirty = false;
      logic(ResourceLogic::Update(resource, source)).unwrap_update()
    } else {
      resource
    }
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
    &self.data.get(&source.id).unwrap().0
  }
}
