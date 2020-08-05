use rendiation_mesh_buffer::geometry::IndexedGeometry;
use rendiation_render_entity::*;
use specs::prelude::*;

struct BoundingComponent(pub BoundingData);

impl Component for BoundingComponent {
  type Storage = VecStorage<Self>;
}

struct HitVolumeComponent(pub IndexedGeometry);

impl Component for HitVolumeComponent {
  type Storage = VecStorage<Self>;
}

struct IndexBufferComponent();
impl Component for IndexBufferComponent {
  type Storage = VecStorage<Self>;
}

struct IndexBufferUpdateSourceComponent();
impl Component for IndexBufferUpdateSourceComponent {
  type Storage = VecStorage<Self>;
}

struct VertexBufferComponent();
impl Component for VertexBufferComponent {
  type Storage = VecStorage<Self>;
}

struct VertexBufferUpdateSourceComponent();
impl Component for VertexBufferUpdateSourceComponent {
  type Storage = VecStorage<Self>;
}
struct GeoemtryGPUUpdateSystem {
  index_updated_entity: Vec<Entity>,
  vertex_updated_entity: Vec<Entity>
}

#[derive(SystemData)]
struct GeoemtryUpdateSysData<'a> {
  index: ReadStorage<'a, IndexBufferComponent>,
  index_to_update: WriteStorage<'a, IndexBufferUpdateSourceComponent>,
  vertex: ReadStorage<'a, VertexBufferComponent>,
  vertex_to_update: WriteStorage<'a, VertexBufferUpdateSourceComponent>,
  entities: Entities<'a>,
}

impl<'a> System<'a> for GeoemtryGPUUpdateSystem {
  type SystemData = GeoemtryUpdateSysData<'a>;

  fn run(&mut self, mut data: GeoemtryUpdateSysData) {
    // // Join merges the two component storages,
    // // so you get all (CompInt, CompBool) pairs.
    for (index, index_to_update) in (&data.index, &mut data.index_to_update).join() {
        // cb.0 = ci.0 > 0;
    }
  }
}

#[test]
fn test() {
  let mut w = World::new();

  let geometry = w
    .create_entity()
    .with(BoundingComponent(BoundingData::empty()))
    .with(HitVolumeComponent(IndexedGeometry::new(vec![], vec![])))
    .build();

  w.maintain();
  //   w.write_component()
}
