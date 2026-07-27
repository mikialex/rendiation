use crate::*;

/// Encodes a unit normal vector to octahedral 8:8 format packed into u32.
///
/// The encoding projects the unit normal onto an octahedron via L1 normalization,
/// then folds the back hemisphere (z < 0) into the front. The resulting 2D coordinates
/// are quantized to 8-bit unorm each and packed into the low 16 bits of a u32
/// (`pack4x8unorm` compatible layout: byte 0 = x, byte 1 = y, bytes 2,3 = 0).
pub fn encode_octahedral_normal(normal: Vec3<f32>) -> u32 {
  let n = normal.normalize();
  let d = n.x.abs() + n.y.abs() + n.z.abs();
  let mut x = n.x / d;
  let mut y = n.y / d;

  // fold when z < 0
  if n.z < 0.0 {
    let sx = if x >= 0.0 { 1.0 } else { -1.0 };
    let sy = if y >= 0.0 { 1.0 } else { -1.0 };
    let nx = (1.0 - y.abs()) * sx;
    let ny = (1.0 - x.abs()) * sy;
    x = nx;
    y = ny;
  }

  // map from [-1, 1] to [0, 1] and quantize to 8-bit unorm
  let u = ((x * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u32;
  let v = ((y * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u32;
  // pack4x8unorm layout: byte 0 = x, byte 1 = y, bytes 2,3 = 0
  u | (v << 8)
}

#[shader_fn]
pub fn decode_octahedral_normal(packed: Node<u32>) -> Node<Vec3<f32>> {
  let uv = packed.unpack4x8unorm().xy();
  let f = uv * val(Vec2 { x: 2.0, y: 2.0 }) - val(Vec2 { x: 1.0, y: 1.0 });

  let z = val(1.0) - f.x().abs() - f.y().abs();
  let sx = f
    .x()
    .greater_equal_than(val(0.0))
    .select(val(1.0), val(-1.0));
  let sy = f
    .y()
    .greater_equal_than(val(0.0))
    .select(val(1.0), val(-1.0));
  let snz = vec2_node((sx, sy));
  let unfolded = vec2_node((
    (val(1.0) - f.y().abs()) * snz.x(),
    (val(1.0) - f.x().abs()) * snz.y(),
  ));

  let p = z.less_than(val(0.0)).select(unfolded, f);
  vec3_node((p.x(), p.y(), z)).normalize()
}

/// CPU-side octahedral 8:8 decode.
pub fn decode_octahedral_normal_cpu(packed: u32) -> Vec3<f32> {
  let u = (packed & 0xFF) as f32 / 255.0;
  let v = ((packed >> 8) & 0xFF) as f32 / 255.0;

  let mut x = u * 2.0 - 1.0;
  let mut y = v * 2.0 - 1.0;

  let z = 1.0 - x.abs() - y.abs();

  if z < 0.0 {
    let sx = if x >= 0.0 { 1.0 } else { -1.0 };
    let sy = if y >= 0.0 { 1.0 } else { -1.0 };
    let nx = (1.0 - y.abs()) * sx;
    let ny = (1.0 - x.abs()) * sy;
    x = nx;
    y = ny;
  }

  Vec3 { x, y, z }.normalize()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn run_roundtrip(normal: Vec3<f32>) {
    let packed = encode_octahedral_normal(normal);
    let decoded = decode_octahedral_normal_cpu(packed);

    let dot = normal.x * decoded.x + normal.y * decoded.y + normal.z * decoded.z;
    assert!(
      dot > 0.99,
      "normal: {:?}, decoded: {:?}, dot: {}",
      normal,
      decoded,
      dot
    );
  }

  #[test]
  fn octahedral_8x8_px() {
    run_roundtrip(Vec3 {
      x: 1.0,
      y: 0.0,
      z: 0.0,
    });
  }

  #[test]
  fn octahedral_8x8_nx() {
    run_roundtrip(Vec3 {
      x: -1.0,
      y: 0.0,
      z: 0.0,
    });
  }

  #[test]
  fn octahedral_8x8_py() {
    run_roundtrip(Vec3 {
      x: 0.0,
      y: 1.0,
      z: 0.0,
    });
  }

  #[test]
  fn octahedral_8x8_ny() {
    run_roundtrip(Vec3 {
      x: 0.0,
      y: -1.0,
      z: 0.0,
    });
  }

  #[test]
  fn octahedral_8x8_pz() {
    run_roundtrip(Vec3 {
      x: 0.0,
      y: 0.0,
      z: 1.0,
    });
  }

  #[test]
  fn octahedral_8x8_nz() {
    run_roundtrip(Vec3 {
      x: 0.0,
      y: 0.0,
      z: -1.0,
    });
  }

  #[test]
  fn octahedral_8x8_diagonal() {
    run_roundtrip(
      Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
      }
      .normalize(),
    );
  }

  #[test]
  fn octahedral_8x8_arbitrary() {
    run_roundtrip(
      Vec3 {
        x: -0.3,
        y: 0.7,
        z: 0.5,
      }
      .normalize(),
    );
  }

  #[test]
  fn octahedral_8x8_edge_grazing() {
    run_roundtrip(
      Vec3 {
        x: 0.99,
        y: 0.01,
        z: 0.1,
      }
      .normalize(),
    );
  }
}
