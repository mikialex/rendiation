pub struct BindGroupManager {
  data: HashMap<TypeId, Box<dyn BindgroupStorageTrait<T>>>,
}

pub struct BindgroupStorage {
  storage: Vec<U>,
}
