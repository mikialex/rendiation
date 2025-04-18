use crate::*;

/// The most common storage type that use a vec as the container.
/// Expecting dense distributed component data
pub struct DBDefaultLinearStorage<T> {
  pub data: Vec<T>,
  pub default_value: T,
}

impl<T: CValue> ComponentStorage for Arc<RwLock<DBDefaultLinearStorage<T>>> {
  fn create_read_view(&self) -> Box<dyn ComponentStorageReadView> {
    Box::new(self.make_read_holder())
  }

  fn create_read_write_view(&self) -> Box<dyn ComponentStorageReadWriteView> {
    Box::new(self.make_write_holder())
  }
  fn type_id(&self) -> TypeId {
    TypeId::of::<T>()
  }
  fn data_shape(&self) -> &'static Shape {
    // T::SHAPE
    unimplemented!()
  }
}

impl<T: CValue> ComponentStorageReadView for LockReadGuardHolder<DBDefaultLinearStorage<T>> {
  fn get(&self, idx: u32) -> Option<DataPtr> {
    self
      .deref()
      .data
      .get(idx as usize)
      .map(|r| r as *const _ as DataPtr)
  }
  fn debug_value(&self, idx: u32) -> Option<String> {
    let data = self.get(idx)?;
    let data = unsafe { &*(data as *const T) };
    format!("{:#?}", data).into()
  }
}

impl<T: CValue> ComponentStorageReadWriteView for LockWriteGuardHolder<DBDefaultLinearStorage<T>> {
  fn notify_start_mutation(&mut self, event: &mut Source<ChangePtr>) {
    let message = ScopedValueChange::<T>::Start;
    event.emit(&(&message as *const _ as ChangePtr));
  }
  fn notify_end_mutation(&mut self, event: &mut Source<ChangePtr>) {
    let message = ScopedValueChange::<T>::End;
    event.emit(&(&message as *const _ as ChangePtr));
  }

  fn get(&self, idx: u32) -> Option<DataPtr> {
    let data: &Vec<T> = &self.data;
    data.get(idx as usize).map(|r| r as *const _ as DataPtr)
  }

  fn set_value(
    &mut self,
    idx: RawEntityHandle,
    v: DataPtr,
    is_create: bool,
    event: &mut Source<ChangePtr>,
  ) -> bool {
    if let Some(target) = self.data.get_mut(idx.index() as usize) {
      let (target, source) = unsafe {
        let target = &mut *(target as *mut T);
        let source = &*(v as *const T);
        (target, source)
      };

      if is_create {
        *target = (*source).clone();

        let change = ValueChange::Delta(source.clone(), None);
        let change = IndexValueChange { idx, change };
        let msg = ScopedValueChange::Message(change);
        event.emit(&(&msg as *const _ as ChangePtr));
      } else if target != source {
        let previous = target.clone();
        let change = ValueChange::Delta(source.clone(), Some(previous));
        *target = (*source).clone();

        let change = IndexValueChange { idx, change };
        let msg = ScopedValueChange::Message(change);
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
    self.set_value(
      idx,
      &self.default_value as *const _ as DataPtr,
      is_create,
      event,
    )
  }

  fn delete(&mut self, idx: RawEntityHandle, event: &mut Source<ChangePtr>) {
    let previous = self.get(idx.index()).unwrap();
    let previous: T = unsafe { (*(previous as *const T)).clone() };
    let change = ValueChange::Remove(previous);
    let change = IndexValueChange { idx, change };
    let msg = ScopedValueChange::Message(change);
    event.emit(&(&msg as *const _ as ChangePtr));
  }

  fn grow(&mut self, max: u32) {
    let max = max as usize;
    if self.data.len() <= max {
      let default = self.default_value.clone();
      self.data.resize(max + 1, default);
    }
  }

  fn debug_value(&self, idx: u32) -> Option<String> {
    let data = self.get(idx)?;
    let data = unsafe { &*(data as *const T) };
    format!("{:#?}", data).into()
  }
}
