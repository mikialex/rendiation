pub trait MutationWatchable {
  type Delta;
  type ModifyView<'a>
  where
    Self: 'a;

  fn create_modify_view<'a>(
    &'a mut self,
    delta: &'a mut Option<Self::Delta>,
  ) -> Self::ModifyView<'a>;
}

struct DataAndDelta<'a, T: MutationWatchable> {
  data: &'a T,
  delta: &'a T::Delta,
}

struct MutationWatcher<T: MutationWatchable> {
  data: T,
  delta: Option<T::Delta>,
}

impl<T: MutationWatchable> MutationWatcher<T> {
  pub fn create_modify_view<'a>(&'a mut self) -> T::ModifyView<'a> {
    self.data.create_modify_view(&mut self.delta)
  }
}

pub struct AtomicModify<'a, T: MutationWatchable> {
  delta: &'a mut Option<T>,
  data: &'a mut T,
}

impl<'a, T: MutationWatchable<Delta = T> + Clone> AtomicModify<'a, T> {
  pub fn set(&mut self, v: T) {
    *self.data = v.clone();
    *self.delta = Some(v);
  }
}

impl MutationWatchable for usize {
  type Delta = usize;
  type ModifyView<'a> = AtomicModify<'a, usize>;

  fn create_modify_view<'a>(
    &'a mut self,
    delta: &'a mut Option<Self::Delta>,
  ) -> Self::ModifyView<'a> {
    AtomicModify { delta, data: self }
  }
}

struct SomeStruct {
  a: usize,
  b: usize,
}

#[derive(Default)]
struct SomeStructDelta {
  a_change: Option<usize>,
  b_change: Option<usize>,
}

struct SomeStructModifyView<'a> {
  data: &'a mut SomeStruct,
  delta: &'a mut Option<SomeStructDelta>,
}

impl<'a> SomeStructModifyView<'a> {
  pub fn set(&mut self, v: SomeStruct) {
    self.set_a(v.a);
    self.set_b(v.b);
  }

  pub fn set_a(&mut self, v: usize) {
    let delta = self.delta.get_or_insert_with(Default::default);
    let mut a = self.data.a.create_modify_view(&mut delta.a_change);
    a.set(v)
  }
  pub fn set_b(&mut self, v: usize) {
    let delta = self.delta.get_or_insert_with(Default::default);
    let mut b = self.data.b.create_modify_view(&mut delta.b_change);
    b.set(v)
  }
}

impl MutationWatchable for SomeStruct {
  type Delta = SomeStructDelta;

  type ModifyView<'a> = SomeStructModifyView<'a>
        where
          Self: 'a;

  fn create_modify_view<'a>(
    &'a mut self,
    delta: &'a mut Option<Self::Delta>,
  ) -> Self::ModifyView<'a> {
    todo!()
  }
}

struct SomeStruct2 {
  a: usize,
  st: SomeStruct,
}

#[derive(Default)]
struct SomeStruct2Delta {
  a_change: Option<usize>,
  st_change: Option<<SomeStruct as MutationWatchable>::Delta>,
}

struct SomeStruct2ModifyView<'a> {
  data: &'a mut SomeStruct2,
  delta: &'a mut Option<SomeStruct2Delta>,
}

impl<'a> SomeStruct2ModifyView<'a> {
  pub fn set(&mut self, v: SomeStruct2) {
    self.set_a(v.a);
    self.set_st(v.st);
  }

  pub fn set_a(&mut self, v: usize) {
    let delta = self.delta.get_or_insert_with(Default::default);
    let mut a = self.data.a.create_modify_view(&mut delta.a_change);
    a.set(v)
  }
  pub fn set_st(&mut self, v: SomeStruct) {
    let mut st = self.modify_st();
    st.set(v)
  }

  pub fn modify_st<'s>(&'s mut self) -> <SomeStruct as MutationWatchable>::ModifyView<'s> {
    let delta = self.delta.get_or_insert_with(Default::default);
    self.data.st.create_modify_view(&mut delta.st_change)
  }
}

pub struct OptionModifyView<'a, T: MutationWatchable> {
  data: &'a mut Option<T>,
  delta: &'a mut Option<Option<T::Delta>>,
}

impl<T: MutationWatchable> MutationWatchable for Option<T> {
  type Delta = Option<T::Delta>;

  type ModifyView<'a>= OptionModifyView<'a, T>
    where
      Self: 'a ;

  fn create_modify_view<'a>(
    &'a mut self,
    delta: &'a mut Option<Self::Delta>,
  ) -> Self::ModifyView<'a> {
    todo!()
  }
}
