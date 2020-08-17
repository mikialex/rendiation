struct Entity {
  ids: HashMap<TypeId, usize>,
  shape: EntityShape,
}

struct EntityShape {
  component_ids: HashSet<TypeId>,
}

pub struct Registy {
  entities: Vec<Entity>,
  shape_cache: HashMap<EntityShape, Entity>,
  storage: HashMap<TypeId, Vec<Any>>,
}

impl Registy {
  pub fn new() -> Self {
    Self {
      entities: Vec::new(),
      shape_cache: HashMap::new(),
      storage: HashMap::new(),
    }
  }
}

// #[test]
fn test() {
  let registry = Registry::new();

  let index_buffer = registy.entity_builder().with(index_buffer);

  let geometry = registy
    .entity_builder(Geomtry)
    .reference(index_buffer)
    .reference(vertex_buffer)
    .reference(range)
    .insert();

  registy
    .entity_builder()
    .with(parent)
    .with(ProjectionMatrix)
    .with(WorldMatrix)
    .with(LocalMatrix)
    .with(pipeline)
    .reference(geometry)
    .insert();

  registry.watch(change, LocalMatrix, ||{

  });

  registry.read().query(..).iter();
  registry.write().query(..).iter();

  // registry.watch()
  //     .filter(|f|)

  let system = System::new();

  let hierachy_update_system = |r| {
    r.query()
    .component(has(Hierachy).modified())
    .iter()
    .for_each();
  };

  let material = interface()
    .has_ref(pipeline);

  let geometry = interface()
    .has_ref(vertex_buffer)
    .optional_ref(index_buffer)
    .optional_ref(range);

  let drawable = interface()
    .has_ref(material)
    .has_ref(geometry);


}

// let upate = |parent: Option<T>|{

// }
