use crate::*;

impl<T: ReverseIncremental + Default> ReverseIncremental for Vec<T> {
  fn apply_rev(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error> {
    let r = match delta {
      VecDelta::Push(value) => {
        self.push(value);
        VecDelta::Pop
      }
      VecDelta::Remove(index) => {
        let item = self.remove(index);
        VecDelta::Insert(index, item)
      }
      VecDelta::Insert(index, item) => {
        self.insert(index, item);
        VecDelta::Remove(index)
      }
      VecDelta::Pop => {
        let value = self.pop().unwrap();
        VecDelta::Push(value)
      }
      VecDelta::Mutate(index, delta) => {
        let inner = self.get_mut(index).unwrap();
        VecDelta::Mutate(index, inner.apply_rev(delta).unwrap())
      }
    };

    Ok(r)
  }
}

// impl ReverseIncremental for f32 {

//   fn apply(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error> {
//     let old = *self;
//     *self = delta;
//     Ok(old)
//   }
// }

// impl ReverseIncremental for bool {

//   fn apply(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error> {
//     let old = *self;
//     *self = delta;
//     Ok(old)
//   }
// }
