
#[derive(Default)]
pub struct ResourceMapper {
  resources: Vec<Box<dyn Any>>,
}


impl ResourceMapper {
  pub fn get_resource(&mut self, backend: usize) {
    //
  }
}