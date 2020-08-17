#![allow(dead_code)]
#![allow(unused)]

use rendiation_mesh_buffer::geometry::IndexedGeometry;
use rendiation_render_entity::*;
use legion::prelude::*;
use rendiation_math_entity::*;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
  x: f32,
  y: f32,
}

// #[test]
fn test() {
  let universe = Universe::new();
  let mut world = universe.create_world();
  
  world.insert((), (0..999).map(|_| (Position { x: 0.0, y: 0.0 },)));
  world.insert((), (0..999).map(|_| (BoundingData::empty(),)));
  world.insert((), vec![(BoundingData::empty(),)]);
  // let geometry = w
  //   .create_entity()
  //   .with(BoundingComponent(BoundingData::empty()))
  //   .with(HitVolumeComponent(IndexedGeometry::new(vec![], vec![])))
  //   .build();

  // w.maintain();
  //   w.write_component()
}
