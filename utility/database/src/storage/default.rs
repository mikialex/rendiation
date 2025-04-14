use crate::*;

impl<T: CValue + Default> ComponentStorage for Arc<RwLock<Vec<T>>> {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView> {
    Box::new(self.make_read_holder())
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView> {
    Box::new(self.make_write_holder())
  }
}

impl<T: CValue> ComponentStorageReadView for LockReadGuardHolder<Vec<T>> {
  fn get(&self, idx: u32) -> Option<DataPtr> {
    self
      .deref()
      .get(idx as usize)
      .map(|r| r as *const _ as DataPtr)
  }
  fn debug_value(&self, idx: u32) -> Option<String> {
    format!("{:#?}", self.get(idx)?).into()
  }
  fn type_id(&self) -> TypeId {
    TypeId::of::<T>()
  }
}

impl<T: CValue + Default> ComponentStorageReadWriteView for LockWriteGuardHolder<Vec<T>> {
  fn notify_start_mutation(&mut self, event: &mut Source<ChangePtr>) {
    let message = ScopedMessage::<T>::Start;
    event.emit(&(&message as *const _ as ChangePtr));
  }
  fn notify_end_mutation(&mut self, event: &mut Source<ChangePtr>) {
    let message = ScopedMessage::<T>::End;
    event.emit(&(&message as *const _ as ChangePtr));
  }

  fn get(&self, idx: u32) -> Option<DataPtr> {
    let data: &Vec<T> = self;
    data.get(idx as usize).map(|r| r as *const _ as DataPtr)
  }

  fn set_value(
    &mut self,
    idx: RawEntityHandle,
    v: DataPtr,
    is_create: bool,
    event: &mut Source<ChangePtr>,
  ) -> bool {
    if let Some(target) = self.get_mut(idx.index() as usize) {
      let (target, source) = unsafe {
        let target = &mut *(target as *mut T);
        let source = &*(v as *const T);
        (target, source)
      };

      if target != source {
        let change = if is_create {
          ValueChange::Delta(source.clone(), None)
        } else {
          let previous = target.clone();
          ValueChange::Delta(source.clone(), Some(previous))
        };

        *target = (*source).clone();

        let change = IndexValueChange { idx, change };
        let msg = ScopedMessage::Message(change);
        event.emit(&(&msg as *const _ as ChangePtr));
      }

      true
    } else {
      false
    }
  }

  fn set_default_value(
    &mut self,
    idx: RawEntityHandle,
    is_create: bool,
    event: &mut Source<ChangePtr>,
  ) -> bool {
    let value = T::default();
    self.set_value(idx, &value as *const _ as DataPtr, is_create, event)
  }

  fn delete(&mut self, idx: RawEntityHandle, event: &mut Source<ChangePtr>) {
    let previous = self.get(idx.index()).unwrap();
    let previous: T = unsafe { (*(previous as *const T)).clone() };
    let change = ValueChange::Remove(previous);
    let change = IndexValueChange { idx, change };
    let msg = ScopedMessage::Message(change);
    event.emit(&(&msg as *const _ as ChangePtr));
  }

  unsafe fn grow_at_least(&mut self, max: usize) {
    let data: &mut Vec<T> = self;
    if data.len() <= max {
      data.resize(max + 1, T::default());
    }
  }

  fn debug_value(&self, idx: u32) -> Option<String> {
    format!("{:#?}", self.get(idx)?).into()
  }
  fn type_id(&self) -> TypeId {
    TypeId::of::<T>()
  }
}
