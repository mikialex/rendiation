pub struct Entity {}

pub struct Registry {
  entities: Arena<Entity>,
  components: HashMap<TypeId, Arena<Any>>,
}

impl Registry {
  pub fn new() -> Self {}
}

pub fn test() {
  let some_drawable = scene
    .create_entity()
    .set(index_buffer)
    .set(vertex)
    .set(group)
    .set(shading);

  let some_drawable = scene.create_entity().set(geometry).set(group).set(shading);
  // let some_material = scene.create_entity()
  //     .with(bindgroups)
  //     .with(pipeline)
  //     .with();

  let object = scene
    .create_entity()
    .set(some_hierarchy)
    .set(some_hit_volume)
    .set(some_culling_bound)
    .set(some_drawable);

  scene.get_entity(object).set(other_drawable);

  let process_pick = graph().process().on::<PickEvent>().work(|| {
    // do pick
    // foreach entity<require(some_hit_bound), optional(some_culling_bound)>
  });

  let list_gen = graph
    .process()
    .read_write::<(Read<A>, Write<B>)>()
    .work(|dep| {
      // do work
    });
}
