pub trait IndexedSeparatedGeometry {}

pub struct IndexedSeparatedAnyGeometry {
  // pub attributes: HashMap<String, V>
  pub position: Vec<Vec3<f32>>,
  pub normal: Vec<Vec3<f32>>,
  pub uv: Vec<Vec2<f32>>,

  pub index: Vec<u16>,
}

// struct StructOfArray<T> {
//   layout: PhantomData<T>,
// }

pub trait StructOfArray {
  type Data;
}

// use marco to generate this;
struct VertexStructOfArrayInstance {
  pub position: Vec<Vec3<f32>>,
  pub normal: Vec<Vec3<f32>>,
  pub uv: Vec<Vec2<f32>>,
}

impl StructOfArray for Vertex {
  type Data = VertexStructOfArrayInstance;
}
