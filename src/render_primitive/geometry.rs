
// trait RayCasterable {
//     fn hitFirst() -> 
// }

pub struct Geometry {
    pub bounding_box: Box3,
    pub bounding_sphere: Sphere,
    pub id: usize,

    pub index: Option<Rc<BufferData<u16>>>,

    pub attributes: HashMap<String, Rc<BufferData<f32>>>,
}
