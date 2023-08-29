use std::hash::{BuildHasherDefault, Hash, Hasher};

#[derive(Clone)]
pub struct VertexPosition(pub [f32; 3]);

impl VertexPosition {
  const BYTES: usize = 3 * std::mem::size_of::<f32>();

  fn as_bytes(&self) -> &[u8] {
    let bytes: &[u8; Self::BYTES] = unsafe { std::mem::transmute(&self.0) };
    bytes
  }
}

impl Hash for VertexPosition {
  fn hash<H: Hasher>(&self, state: &mut H) {
    state.write(self.as_bytes());
  }
}

impl PartialEq for VertexPosition {
  fn eq(&self, other: &Self) -> bool {
    self.as_bytes() == other.as_bytes()
  }
}

impl Eq for VertexPosition {}

#[derive(Default)]
pub struct PositionHasher {
  state: u64,
}

impl Hasher for PositionHasher {
  fn write(&mut self, bytes: &[u8]) {
    assert!(bytes.len() == VertexPosition::BYTES);

    let a = u32::from_ne_bytes((&bytes[0..4]).try_into().unwrap());
    let b = u32::from_ne_bytes((&bytes[4..8]).try_into().unwrap());
    let c = u32::from_ne_bytes((&bytes[8..12]).try_into().unwrap());

    // scramble bits to make sure that integer coordinates have entropy in lower bits
    let a = a ^ (a >> 17);
    let b = b ^ (b >> 17);
    let c = c ^ (c >> 17);

    // Optimized Spatial Hashing for Collision Detection of Deformable Objects
    self.state =
      ((a.wrapping_mul(73856093)) ^ (b.wrapping_mul(19349663)) ^ (c.wrapping_mul(83492791))) as u64;
  }

  fn finish(&self) -> u64 {
    self.state
  }
}

pub type BuildPositionHasher = BuildHasherDefault<PositionHasher>;

// #[derive(Default)]
// pub struct IdHasher {
//   state: u64,
// }

// impl Hasher for IdHasher {
//   fn write(&mut self, bytes: &[u8]) {
//     assert!(bytes.len() == std::mem::size_of::<u32>());

//     let mut h = u32::from_ne_bytes((&bytes[0..4]).try_into().unwrap());

//     // MurmurHash2 finalizer
//     h ^= h >> 13;
//     h = h.wrapping_mul(0x5bd1e995);
//     h ^= h >> 15;

//     self.state = h as u64;
//   }

//   fn finish(&self) -> u64 {
//     self.state
//   }
// }

// pub type BuildIdHasher = BuildHasherDefault<IdHasher>;
