use crate::*;

impl<P: Simplex> FromIterator<P> for NoneIndexedMesh<P::Topology, Vec<P::Vertex>> {
  fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
    let iter = iter.into_iter();
    NoneIndexedMesh::new(iter.flatten().collect())
  }
}

impl<P: Simplex> FromIterator<P> for IndexedMesh<P::Topology, Vec<P::Vertex>, Vec<u32>>
where
  P::Vertex: std::hash::Hash + Eq + Copy,
{
  fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
    let mut deduplicate = FastHashMap::<P::Vertex, u32>::default();
    let iter = iter.into_iter();

    let mut vertices: Vec<P::Vertex> = Vec::with_capacity(iter.size_hint().0 * P::DIMENSION);

    let push_v = |v: P::Vertex| {
      *deduplicate.entry(v).or_insert_with(|| {
        vertices.push(v);
        vertices.len() as u32 - 1
      })
    };

    let indices = iter.flat_map(|p| p.into_iter()).map(push_v).collect();
    vertices.shrink_to_fit();

    IndexedMesh::new(vertices, indices)
  }
}

pub trait AttributeVertex {
  fn layout(&self) -> Vec<AttributeSemantic>;
  fn write(self, target: &mut [Vec<u8>]);
}

impl AttributeVertex for FullReaderRead<'_> {
  fn write(self, target: &mut [Vec<u8>]) {
    for (source, target) in self.read_bytes().zip(target.iter_mut()) {
      target.extend_from_slice(source)
    }
  }

  fn layout(&self) -> Vec<AttributeSemantic> {
    self.reader.keys.clone()
  }
}

impl<P: Simplex> FromIterator<P> for AttributesMeshData
where
  P::Vertex: std::hash::Hash + Eq + Copy + AttributeVertex,
{
  fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
    let mut deduplicate = FastHashMap::<P::Vertex, u32>::default();
    let iter = iter.into_iter();

    let vertex_max_count = iter.size_hint().0 * P::DIMENSION;

    let mut write_count = 0;
    let mut buffers: Option<Vec<Vec<u8>>> = None;
    let mut layout: Option<Vec<AttributeSemantic>> = None;

    let push_v = |v: P::Vertex| {
      *deduplicate.entry(v).or_insert_with(|| {
        let buffers = buffers.get_or_insert_with(|| {
          layout
            .get_or_insert_with(|| v.layout())
            .iter()
            .map(|k| Vec::with_capacity(vertex_max_count * k.item_byte_size()))
            .collect()
        });

        v.write(buffers);
        write_count += 1;
        write_count as u32 - 1
      })
    };

    let indices: Vec<u32> = iter.flat_map(|p| p.into_iter()).map(push_v).collect();
    let indices: Vec<u8> = bytemuck::cast_slice(&indices).to_owned();

    let attributes = buffers
      .unwrap()
      .into_iter()
      .zip(layout.unwrap())
      .map(|(buffer, s)| (s, buffer))
      .collect();

    AttributesMeshData {
      attributes,
      indices: Some((AttributeIndexFormat::Uint32, indices)),
      mode: P::TOPOLOGY,
      groups: Default::default(),
    }
  }
}
