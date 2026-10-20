//! `.pnm` file format: the 4 byte magic `PNM\0`, the format version as little endian `u32`,
//! then the `ParametricNormalizedModel` encoded as MessagePack (compact, structs as arrays).
//! The compact encoding depends on field order, so any change to a serialized type must bump
//! `PNM_FORMAT_VERSION`.

use std::io::{Read, Write};
use std::path::Path;

use crate::*;

pub const PNM_EXTENSION: &str = "pnm";
pub const PNM_FORMAT_VERSION: u32 = 1;
const PNM_MAGIC: [u8; 4] = *b"PNM\0";

impl ParametricNormalizedModel {
  pub fn write_pnm(&self, mut writer: impl Write) -> Result<(), PnmError> {
    writer.write_all(&PNM_MAGIC)?;
    writer.write_all(&PNM_FORMAT_VERSION.to_le_bytes())?;
    rmp_serde::encode::write(&mut writer, self)?;
    Ok(())
  }

  /// Decodes and checks the structural invariants (`check_structure`), so the result can be
  /// indexed without bound checks.
  pub fn read_pnm(mut reader: impl Read) -> Result<Self, PnmError> {
    let mut header = [0; 8];
    reader.read_exact(&mut header)?;
    if header[..4] != PNM_MAGIC {
      return Err(PnmError::NotPnm);
    }
    let version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    if version != PNM_FORMAT_VERSION {
      return Err(PnmError::UnsupportedVersion(version));
    }
    let model: Self = rmp_serde::decode::from_read(reader)?;
    model.check_structure().map_err(PnmError::Invalid)?;
    Ok(model)
  }

  pub fn save_pnm(&self, path: impl AsRef<Path>) -> Result<(), PnmError> {
    let mut writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    self.write_pnm(&mut writer)?;
    writer.flush()?;
    Ok(())
  }

  pub fn load_pnm(path: impl AsRef<Path>) -> Result<Self, PnmError> {
    Self::read_pnm(std::io::BufReader::new(std::fs::File::open(path)?))
  }

  /// Index and size invariants every consumer relies on: control point counts match the
  /// degrees, curves are not empty, and all face, edge and patch references are in range.
  pub fn check_structure(&self) -> Result<(), String> {
    for (i, patch) in self.patches.iter().enumerate() {
      let s = &patch.surface;
      let expected = (s.degree_u + 1).checked_mul(s.degree_v + 1);
      if expected != Some(s.control_points.len()) {
        return Err(format!(
          "patch {i}: control point count does not match degrees"
        ));
      }
      if patch.face.index() >= self.faces.len() {
        return Err(format!("patch {i}: face {} out of range", patch.face.0));
      }
      for curve in patch.loops.iter().flat_map(|l| &l.curves) {
        if curve.curve.control_points.is_empty() {
          return Err(format!("patch {i}: empty trim curve"));
        }
        if let TrimCurveSource::Edge { edge, .. } = curve.source
          && edge.index() >= self.edges.len()
        {
          return Err(format!("patch {i}: edge {} out of range", edge.0));
        }
      }
    }
    for (i, instance) in self.instances.iter().enumerate() {
      if instance.face.index() >= self.faces.len() {
        return Err(format!(
          "instance {i}: face {} out of range",
          instance.face.0
        ));
      }
      if instance.patches.start > instance.patches.end || instance.patches.end > self.patches.len()
      {
        return Err(format!("instance {i}: patch range out of range"));
      }
    }
    Ok(())
  }
}

#[derive(Debug)]
pub enum PnmError {
  Io(std::io::Error),
  NotPnm,
  UnsupportedVersion(u32),
  Encode(rmp_serde::encode::Error),
  Decode(rmp_serde::decode::Error),
  Invalid(String),
}

impl std::fmt::Display for PnmError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      PnmError::Io(e) => write!(f, "io error: {e}"),
      PnmError::NotPnm => write!(f, "not a pnm file"),
      PnmError::UnsupportedVersion(v) => write!(
        f,
        "unsupported pnm format version {v} (expected {PNM_FORMAT_VERSION})"
      ),
      PnmError::Encode(e) => write!(f, "pnm encode error: {e}"),
      PnmError::Decode(e) => write!(f, "pnm decode error: {e}"),
      PnmError::Invalid(msg) => write!(f, "invalid pnm content: {msg}"),
    }
  }
}

impl std::error::Error for PnmError {}

impl From<std::io::Error> for PnmError {
  fn from(e: std::io::Error) -> Self {
    PnmError::Io(e)
  }
}

impl From<rmp_serde::encode::Error> for PnmError {
  fn from(e: rmp_serde::encode::Error) -> Self {
    PnmError::Encode(e)
  }
}

impl From<rmp_serde::decode::Error> for PnmError {
  fn from(e: rmp_serde::decode::Error) -> Self {
    PnmError::Decode(e)
  }
}

#[cfg(test)]
mod tests {
  use rendiation_algebra::*;

  use super::*;

  fn sample_model() -> ParametricNormalizedModel {
    let h = std::f64::consts::FRAC_1_SQRT_2;
    let control_points = (0..2)
      .flat_map(|j| {
        [
          Vec4::new(1., 0., j as f64, 1.),
          Vec4::new(h, h, j as f64 * h, h),
          Vec4::new(0., 1., j as f64, 1.),
        ]
      })
      .collect();
    let corners = [
      Vec2::new(0., 0.),
      Vec2::new(1., 0.),
      Vec2::new(1., 1.),
      Vec2::new(0., 1.),
    ];
    let curves = (0..4)
      .map(|i| TrimCurve {
        curve: RationalBezierCurve2::line(corners[i], corners[(i + 1) % 4]),
        source: if i == 0 {
          TrimCurveSource::Edge {
            edge: EdgeId(0),
            reversed: true,
          }
        } else {
          TrimCurveSource::PatchBoundary
        },
        exact: true,
      })
      .collect();
    ParametricNormalizedModel {
      patches: vec![TrimmedBezierPatch {
        surface: RationalBezierSurface::new(2, 1, control_points),
        loops: vec![TrimLoop { curves }],
        flip_normal: true,
        face: FaceId(0),
        cell: (1, 2),
      }],
      instances: vec![FaceInstance {
        face: FaceId(0),
        patches: 0..1,
        transform: Mat4::translate((1., 2., 3.)),
        flip_normal: false,
      }],
      faces: vec![FaceInfo { source_id: 42 }],
      edges: vec![EdgeInfo { source_id: 7 }],
      tolerance: 1e-3,
      length_unit_mm: Some(1.),
    }
  }

  #[test]
  fn pnm_round_trip() {
    let model = sample_model();
    let mut bytes = Vec::new();
    model.write_pnm(&mut bytes).unwrap();
    let read = ParametricNormalizedModel::read_pnm(bytes.as_slice()).unwrap();
    assert_eq!(read, model);
  }

  #[test]
  fn pnm_rejects_bad_header_and_invalid_content() {
    let mut bytes = Vec::new();
    sample_model().write_pnm(&mut bytes).unwrap();

    let mut wrong_magic = bytes.clone();
    wrong_magic[0] = b'X';
    let result = ParametricNormalizedModel::read_pnm(wrong_magic.as_slice());
    assert!(matches!(result, Err(PnmError::NotPnm)));

    let mut wrong_version = bytes.clone();
    wrong_version[4] = 99;
    let result = ParametricNormalizedModel::read_pnm(wrong_version.as_slice());
    assert!(matches!(result, Err(PnmError::UnsupportedVersion(99))));

    let result = ParametricNormalizedModel::read_pnm(&bytes[..bytes.len() / 2]);
    assert!(matches!(result, Err(PnmError::Decode(_))));

    let mut invalid = sample_model();
    invalid.instances[0].patches = 0..2;
    let mut bytes = Vec::new();
    invalid.write_pnm(&mut bytes).unwrap();
    let result = ParametricNormalizedModel::read_pnm(bytes.as_slice());
    assert!(matches!(result, Err(PnmError::Invalid(_))));
  }
}
