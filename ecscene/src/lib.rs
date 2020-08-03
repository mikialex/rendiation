pub struct Entity {}

pub struct Registry {
  entities: Arena<Entity>,
  components: HashMap<TypeId, Arena<Any>>,
}

pub enum EntityFilterKey {
  Optional,
  Require,
  Refuse,
}

pub struct EntityType {
  types: Vec<(TypeId, EntityFilterKey)>,
}

impl Registry {
  pub fn new() -> Self {
    todo!()
  }

  pub fn iter_entity(ty: EntityType) {
    todo!()
  }
}

pub struct ECScene {
  registry: Registry,
  update_system: SceneUpdateSystem,
  culling_system: CullingSystem,
}

pub struct WorkGraph {
  //
}

pub fn test() {
  let geometry = scene
    .create_entity()
    .set(index_buffer)
    .set(vertex_buffers)
    .set(group);

  let shading = scene
    .create_entity()
    .set(target_states)
    .set(ras_states)
    .set();

  let shader = scene.create_entity().set();

  let scene_node_root = scene.create_entity().set(transform);
  let scene_node = scene
    .create_entity()
    .set(transform)
    .set(parent(scene_node_root));

  let some_drawable = scene.create_entity().set(geometry).set(shading);

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
