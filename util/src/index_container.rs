pub struct IndexContainer<T> {
  data: Vec<Option<T>>,
  tomb_list: Vec<usize>,
}

impl<T> IndexContainer<T> {
  pub fn new() -> IndexContainer<T> {
    IndexContainer {
      data: Vec::new(),
      tomb_list: Vec::new(),
    }
  }

  pub fn get_mut(&mut self, index: usize) -> &mut T {
    if let Some(data) = &mut self.data[index] {
      data
    } else {
      panic!("try get a deleted item in array container")
    }
  }

  pub fn get(&self, index: usize) -> &T {
    if let Some(data) = &self.data[index] {
      data
    } else {
      panic!("try get a deleted item in array container")
    }
  }

  pub fn set_item(&mut self, item: T) -> usize {
    let free_index = self.get_free_index();
    if free_index >= self.data.len() {
      self.data.push(Some(item));
    } else {
      self.data[free_index] = Some(item);
    }
    free_index
  }

  fn get_free_index(&mut self) -> usize {
    let free_index;
    if let Some(i) = self.tomb_list.pop() {
      free_index = i;
    } else {
      free_index = self.data.len();
    }
    free_index
  }

  pub fn delete_item(&mut self, index: usize) {
    self.data[index] = None;
    self.tomb_list.push(index);
  }
}

// struct IndexContainerIter<'a, T> {
//   container: &'a mut IndexContainer<T>,
//   cursor: usize,
// }

// impl<'a, T> Iterator for IndexContainerIter<'a, T> {
//   type Item = &'a mut T;

//   fn next(&'a mut self) -> Option<&'a mut T> {
//     if self.cursor == self.container.data.len() {
//       return None;
//     }
//     // let container_data = &mut self.container.data;
//     let mut result = &mut self.container.data[self.cursor];
//     while result.is_some() {
//       self.cursor+=1;
//       result = &mut self.container.data[self.cursor];
//     }
//     result.as_mut()
//   }
// }
