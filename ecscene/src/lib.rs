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
struct IndexBufferUpdateSourceComponent();
struct VertexBufferComponent();
struct VertexBufferUpdateSourceComponent();

// struct GeoemtryGPUUpdateSystem {}

// #[derive(SystemData)]
// struct IntAndBoolData<'a> {
//     comp_int: ReadStorage<'a, CompInt>,
//     comp_bool: WriteStorage<'a, CompBool>,
// }

// impl<'a> System<'a> for GeoemtryGPUUpdateSystem {
//     type SystemData = IntAndBoolData<'a>;

//     fn run(&mut self, mut data: IntAndBoolData) {
//         // Join merges the two component storages,
//         // so you get all (CompInt, CompBool) pairs.
//         for (ci, cb) in (&data.comp_int, &mut data.comp_bool).join() {
//             cb.0 = ci.0 > 0;
//         }
//     }
// }

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
