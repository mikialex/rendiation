pub struct Cosmos {
  entities: Vec<Entity>,
  components: HashMap<TypeId, Box<dyn Any>>,
}

pub struct Entity {
  components: Vec<TypeId, usize>,
}

pub trait Query {
  //
}

#[test]
fn test() {
  //
}
