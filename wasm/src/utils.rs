pub fn set_panic_hook() {
  // When the `console_error_panic_hook` feature is enabled, we can call the
  // `set_panic_hook` function at least once during initialization, and then
  // we will get better error messages if our code ever panics.
  //
  // For more details see
  // https://github.com/rustwasm/console_error_panic_hook#readme
  #[cfg(feature = "console_error_panic_hook")]
  console_error_panic_hook::set_once();
}

pub struct ArrayContainer<T>{
  data: Vec<Option<T>>,
  tomb_list: Vec<usize>
}

impl<T> ArrayContainer<T>{
  pub fn new() -> ArrayContainer<T>{
    ArrayContainer{
      data: Vec::new(),
      tomb_list: Vec::new(),
    }
  }

  pub fn get_mut(&mut self, index: usize) -> &mut T {
    if let Some(data) = &mut self.data[index] {
      data
    }else{
      panic!("try get a deleted item in array container")
    }
  }


  pub fn get(&self, index: usize) -> &T {
    if let Some(data) = &self.data[index] {
      data
    }else{
      panic!("try get a deleted item in array container")
    }
  }

  pub fn set_item(&mut self, item: T, index: usize){
    if index >= self.data.len() {
      self.data.push(Some(item));
    }else{
      self.data[index] = Some(item);
    }
  }

  pub fn get_free_index(&mut self) -> usize{
    let free_index;
    if let Some(i) = self.tomb_list.pop() {
      free_index = i;
    } else {
      free_index = self.data.len();
    }
    free_index
  }

  pub fn delete_item(&mut self, index: usize){
    self.data[index] = None;
    self.tomb_list.push(index);
  }
}