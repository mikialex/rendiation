//! STEP → glTF converter.
//!
//! Reads a STEP file, triangulates every trimmed surface and tessellates
//! 3D edge curves, then exports a single `.glb` file.
//!
//! ```sh
//! cargo run -p rendiation-parametric-rendering --example gltf -- input.stp output.glb
//! ```

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use gltf_json as json;
use json::validation::{Checked, USize64};
use rendiation_algebra::*;
use rendiation_parametric_rendering::mesh::{
  triangulate_trimmed_surface, MeshData, TriangulationConfig,
};
use rendiation_parametric_rendering::step::{
  read_parametric_rendering_data_from_step, StepReadConfig,
};
use rendiation_step_reader::step_utils::normalize_step;

// ── Colors ────────────────────────────────────────────────────────────

fn color_for_index(idx: usize) -> [f32; 4] {
  let hue = (idx.wrapping_mul(0x9E37_79B9).wrapping_add(0x7F4A_7C15) % 360) as f32 / 360.0;
  let sat = 0.6;
  let val = 0.7;
  let c = val * sat;
  let x = c * (1.0 - ((hue * 6.0) % 2.0 - 1.0).abs());
  let m = val - c;
  let (r, g, b) = match (hue * 6.0) as u32 {
    0 => (c, x, 0.0),
    1 => (x, c, 0.0),
    2 => (0.0, c, x),
    3 => (0.0, x, c),
    4 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };
  [r + m, g + m, b + m, 1.0]
}

fn to_f32_bytes(data: &[f32]) -> Vec<u8> {
  data.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn to_u32_bytes(data: &[u32]) -> Vec<u8> {
  data.iter().flat_map(|i| i.to_le_bytes()).collect()
}

// ── glTF document builder ────────────────────────────────────────────

struct GltfDoc {
  bin: Vec<u8>,
  buffer_views: Vec<json::buffer::View>,
  accessors: Vec<json::Accessor>,
  meshes: Vec<json::Mesh>,
  materials: Vec<json::Material>,
  nodes: Vec<json::Node>,
  root_children: Vec<u32>,
  extensions_used: std::collections::BTreeSet<String>,
}

impl GltfDoc {
  fn new() -> Self {
    Self {
      bin: Vec::new(),
      buffer_views: Vec::new(),
      accessors: Vec::new(),
      meshes: Vec::new(),
      materials: Vec::new(),
      nodes: Vec::new(),
      root_children: Vec::new(),
      extensions_used: std::collections::BTreeSet::new(),
    }
  }

  fn push_bytes(&mut self, bytes: &[u8]) -> (u32, u32) {
    let off = self.bin.len() as u32;
    self.bin.extend_from_slice(bytes);
    while self.bin.len() % 4 != 0 {
      self.bin.push(0);
    }
    (off, bytes.len() as u32)
  }

  fn float_min_max(data: &[f32]) -> (f32, f32) {
    data
      .iter()
      .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &x| {
        (lo.min(x), hi.max(x))
      })
  }

  fn make_arr_view(
    &mut self,
    off: u32,
    len: u32,
    target: json::buffer::Target,
  ) -> json::Index<json::buffer::View> {
    let idx = self.buffer_views.len() as u32;
    self.buffer_views.push(json::buffer::View {
      buffer: json::Index::new(0),
      byte_offset: Some(USize64(off as u64)),
      byte_length: USize64(len as u64),
      byte_stride: None,
      target: Some(Checked::Valid(target)),
      name: None,
      extensions: None,
      extras: Default::default(),
    });
    json::Index::new(idx)
  }

  fn add_vec3_accessor(&mut self, data: &[Vec3<f32>]) -> json::Index<json::Accessor> {
    let count = data.len() as u32;
    let xs: Vec<f32> = data.iter().map(|v| v.x).collect();
    let ys: Vec<f32> = data.iter().map(|v| v.y).collect();
    let zs: Vec<f32> = data.iter().map(|v| v.z).collect();
    let raw: Vec<f32> = data.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
    let bytes = to_f32_bytes(&raw);
    let (off, len) = self.push_bytes(&bytes);
    let view_idx = self.make_arr_view(off, len, json::buffer::Target::ArrayBuffer);

    let (xmin, xmax) = Self::float_min_max(&xs);
    let (ymin, ymax) = Self::float_min_max(&ys);
    let (zmin, zmax) = Self::float_min_max(&zs);

    let idx = self.accessors.len() as u32;
    self.accessors.push(json::Accessor {
      buffer_view: Some(view_idx),
      byte_offset: Some(USize64(0)),
      count: USize64(count as u64),
      component_type: Checked::Valid(json::accessor::GenericComponentType(
        json::accessor::ComponentType::F32,
      )),
      type_: Checked::Valid(json::accessor::Type::Vec3),
      min: Some(json::Value::Array(vec![
        xmin.into(),
        ymin.into(),
        zmin.into(),
      ])),
      max: Some(json::Value::Array(vec![
        xmax.into(),
        ymax.into(),
        zmax.into(),
      ])),
      name: None,
      normalized: false,
      sparse: None,
      extensions: None,
      extras: Default::default(),
    });
    json::Index::new(idx)
  }

  fn add_vec2_accessor(&mut self, data: &[Vec2<f32>]) -> json::Index<json::Accessor> {
    let count = data.len() as u32;
    let xs: Vec<f32> = data.iter().map(|v| v.x).collect();
    let ys: Vec<f32> = data.iter().map(|v| v.y).collect();
    let raw: Vec<f32> = data.iter().flat_map(|v| [v.x, v.y]).collect();
    let bytes = to_f32_bytes(&raw);
    let (off, len) = self.push_bytes(&bytes);
    let view_idx = self.make_arr_view(off, len, json::buffer::Target::ArrayBuffer);

    let (xmin, xmax) = Self::float_min_max(&xs);
    let (ymin, ymax) = Self::float_min_max(&ys);

    let idx = self.accessors.len() as u32;
    self.accessors.push(json::Accessor {
      buffer_view: Some(view_idx),
      byte_offset: Some(USize64(0)),
      count: USize64(count as u64),
      component_type: Checked::Valid(json::accessor::GenericComponentType(
        json::accessor::ComponentType::F32,
      )),
      type_: Checked::Valid(json::accessor::Type::Vec2),
      min: Some(json::Value::Array(vec![xmin.into(), ymin.into()])),
      max: Some(json::Value::Array(vec![xmax.into(), ymax.into()])),
      name: None,
      normalized: false,
      sparse: None,
      extensions: None,
      extras: Default::default(),
    });
    json::Index::new(idx)
  }

  fn add_u32_index_accessor(&mut self, indices: &[[u32; 3]]) -> json::Index<json::Accessor> {
    let flat: Vec<u32> = indices.iter().flat_map(|t| [t[0], t[1], t[2]]).collect();
    let count = flat.len() as u32;
    let bytes = to_u32_bytes(&flat);
    let (off, len) = self.push_bytes(&bytes);
    let view_idx = self.make_arr_view(off, len, json::buffer::Target::ElementArrayBuffer);

    let idx = self.accessors.len() as u32;
    self.accessors.push(json::Accessor {
      buffer_view: Some(view_idx),
      byte_offset: Some(USize64(0)),
      count: USize64(count as u64),
      component_type: Checked::Valid(json::accessor::GenericComponentType(
        json::accessor::ComponentType::U32,
      )),
      type_: Checked::Valid(json::accessor::Type::Scalar),
      min: None,
      max: None,
      name: None,
      normalized: false,
      sparse: None,
      extensions: None,
      extras: Default::default(),
    });
    json::Index::new(idx)
  }

  fn add_u32_index_accessor_flat(&mut self, indices: &[u32]) -> json::Index<json::Accessor> {
    let count = indices.len() as u32;
    let bytes = to_u32_bytes(indices);
    let (off, len) = self.push_bytes(&bytes);
    let view_idx = self.make_arr_view(off, len, json::buffer::Target::ElementArrayBuffer);

    let idx = self.accessors.len() as u32;
    self.accessors.push(json::Accessor {
      buffer_view: Some(view_idx),
      byte_offset: Some(USize64(0)),
      count: USize64(count as u64),
      component_type: Checked::Valid(json::accessor::GenericComponentType(
        json::accessor::ComponentType::U32,
      )),
      type_: Checked::Valid(json::accessor::Type::Scalar),
      min: None,
      max: None,
      name: None,
      normalized: false,
      sparse: None,
      extensions: None,
      extras: Default::default(),
    });
    json::Index::new(idx)
  }

  fn make_material(&mut self, color: [f32; 4]) -> json::Index<json::Material> {
    let idx = self.materials.len() as u32;
    self.materials.push(json::Material {
      pbr_metallic_roughness: json::material::PbrMetallicRoughness {
        base_color_factor: json::material::PbrBaseColorFactor(color),
        metallic_factor: json::material::StrengthFactor(0.0),
        roughness_factor: json::material::StrengthFactor(0.9),
        ..Default::default()
      },
      ..Default::default()
    });
    json::Index::new(idx)
  }

  fn add_surface_mesh(&mut self, mesh: &MeshData) {
    let pos_acc = self.add_vec3_accessor(&mesh.positions);
    let nrm_acc = self.add_vec3_accessor(&mesh.normals);
    let uv_acc = self.add_vec2_accessor(&mesh.uvs);

    let idx_acc = if !mesh.indices.is_empty() {
      Some(self.add_u32_index_accessor(&mesh.indices))
    } else {
      None
    };

    let mat_idx = self.make_material(color_for_index(self.materials.len()));

    let mut attrs = BTreeMap::new();
    attrs.insert(Checked::Valid(json::mesh::Semantic::Positions), pos_acc);
    attrs.insert(Checked::Valid(json::mesh::Semantic::Normals), nrm_acc);
    attrs.insert(Checked::Valid(json::mesh::Semantic::TexCoords(0)), uv_acc);

    let prim = json::mesh::Primitive {
      attributes: attrs,
      indices: idx_acc,
      material: Some(mat_idx),
      mode: Checked::Valid(json::mesh::Mode::Triangles),
      extensions: None,
      extras: Default::default(),
      targets: None,
    };

    self.meshes.push(json::Mesh {
      primitives: vec![prim],
      name: Some(format!("surface_{}", self.meshes.len())),
      extensions: None,
      extras: Default::default(),
      weights: None,
    });

    let mesh_idx = (self.meshes.len() - 1) as u32;
    let node_idx = self.nodes.len() as u32;
    self.nodes.push(json::Node {
      mesh: Some(json::Index::new(mesh_idx)),
      name: Some(format!("surface_node_{}", self.nodes.len())),
      ..Default::default()
    });
    self.root_children.push(node_idx);
  }

  fn add_curve_mesh(&mut self, points: &[Vec3<f32>], curve_idx: usize, use_line_list: bool) {
    if points.len() < 2 {
      return;
    }
    let pos_acc = self.add_vec3_accessor(points);
    let (line_indices, mode) = if use_line_list {
      // LINE_LIST: each segment is an independent pair (i, i+1)
      let n = points.len();
      let pairs: Vec<u32> = (0..n as u32 - 1)
        .flat_map(|i| [i, i + 1])
        .collect();
      (pairs, json::mesh::Mode::Lines)
    } else {
      let indices: Vec<u32> = (0..points.len() as u32).collect();
      (indices, json::mesh::Mode::LineStrip)
    };
    let idx_acc = self.add_u32_index_accessor_flat(&line_indices);

    self.extensions_used
      .insert("KHR_materials_unlit".to_string());
    let mat_idx = self.materials.len() as u32;
    self.materials.push(json::Material {
      pbr_metallic_roughness: json::material::PbrMetallicRoughness {
        base_color_factor: json::material::PbrBaseColorFactor([0.0, 0.0, 0.0, 1.0]),
        ..Default::default()
      },
      extensions: Some(json::extensions::material::Material {
        unlit: Some(json::extensions::material::Unlit {}),
        ..Default::default()
      }),
      ..Default::default()
    });

    let mut attrs = BTreeMap::new();
    attrs.insert(Checked::Valid(json::mesh::Semantic::Positions), pos_acc);

    let prim = json::mesh::Primitive {
      attributes: attrs,
      indices: Some(idx_acc),
      material: Some(json::Index::new(mat_idx)),
      mode: Checked::Valid(mode),
      extensions: None,
      extras: Default::default(),
      targets: None,
    };

    self.meshes.push(json::Mesh {
      primitives: vec![prim],
      name: Some(format!("curve_{curve_idx}")),
      extensions: None,
      extras: Default::default(),
      weights: None,
    });

    let mesh_idx = (self.meshes.len() - 1) as u32;
    let node_idx = self.nodes.len() as u32;
    self.nodes.push(json::Node {
      mesh: Some(json::Index::new(mesh_idx)),
      name: Some(format!("curve_node_{curve_idx}")),
      ..Default::default()
    });
    self.root_children.push(node_idx);
  }

  fn into_root(self) -> (json::Root, Vec<u8>) {
    let bin = self.bin;
    let root = json::Root {
      accessors: self.accessors,
      asset: json::Asset {
        version: String::from("2.0"),
        ..Default::default()
      },
      buffers: vec![json::Buffer {
        byte_length: USize64(bin.len() as u64),
        uri: None,
        name: None,
        extensions: None,
        extras: Default::default(),
      }],
      buffer_views: self.buffer_views,
      meshes: self.meshes,
      materials: self.materials,
      nodes: self.nodes.clone(),
      scenes: vec![json::Scene {
        nodes: self
          .root_children
          .iter()
          .map(|&i| json::Index::new(i))
          .collect(),
        name: None,
        extensions: None,
        extras: Default::default(),
      }],
      scene: Some(json::Index::new(0)),
      extensions_used: self
        .extensions_used
        .into_iter()
        .collect::<Vec<_>>(),
      ..Default::default()
    };
    (root, bin)
  }
}

// ── GLB writer ──────────────────────────────────────────────────────

fn write_glb(
  path: &Path,
  json_root: &json::Root,
  bin: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
  let json_bytes = serde_json::to_vec(json_root)?;
  let json_padded = {
    let mut v = json_bytes;
    while v.len() % 4 != 0 {
      v.push(0x20);
    }
    v
  };

  let total_len =
    12u32 + 8u32 + json_padded.len() as u32 + if bin.is_empty() { 0 } else { 8 } + bin.len() as u32;

  let mut out = Vec::with_capacity(total_len as usize);
  out.write_all(&0x4654_6C67_u32.to_le_bytes())?; // magic
  out.write_all(&2u32.to_le_bytes())?; // version
  out.write_all(&total_len.to_le_bytes())?;

  out.write_all(&(json_padded.len() as u32).to_le_bytes())?;
  out.write_all(&0x4E4F_534A_u32.to_le_bytes())?; // "JSON"
  out.write_all(&json_padded)?;

  if !bin.is_empty() {
    out.write_all(&(bin.len() as u32).to_le_bytes())?;
    out.write_all(&0x004E_4942_u32.to_le_bytes())?; // "BIN\0"
    out.write_all(bin)?;
  }

  fs::write(path, out)?;
  Ok(())
}

// ── Main ────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();
  if args.len() < 3 {
    eprintln!("usage: {} <input.stp> <output.glb> [--linelist]", args[0]);
    std::process::exit(1);
  }

  let step_path = Path::new(&args[1]);
  let gltf_path = Path::new(&args[2]);
  let use_line_list = args.get(3).map(|s| s.as_str()) == Some("--linelist");

  eprintln!("reading STEP: {}", step_path.display());
  let raw = fs::read_to_string(step_path)?;
  let step_str = normalize_step(&raw);

  let step_config = StepReadConfig::default();
  let data = read_parametric_rendering_data_from_step(&step_str, step_config)?;

  eprintln!(
    "  {} trimmed surfaces, {} 3D curves",
    data.surfaces.len(),
    data.curves_3d.len()
  );

  let tri_config = TriangulationConfig::default();
  let mut doc = GltfDoc::new();

  for (i, trimmed) in data.surfaces.iter().enumerate() {
    eprintln!(
      "  triangulating surface {}/{}{}",
      i + 1,
      data.surfaces.len(),
      if trimmed.trim_boundary.is_empty() {
        " (untrimmed)"
      } else {
        ""
      }
    );
    let mesh = triangulate_trimmed_surface(trimmed, &tri_config);
    if mesh.indices.is_empty() {
      eprintln!("    skipped (empty triangulation)");
      continue;
    }
    eprintln!(
      "    {} vertices, {} triangles",
      mesh.positions.len(),
      mesh.indices.len()
    );
    doc.add_surface_mesh(&mesh);
  }

  if !data.curves_3d.is_empty() {
    eprintln!("  tessellating {} 3D curves...", data.curves_3d.len());
  }
  for (i, curve) in data.curves_3d.iter().enumerate() {
    let pts = rendiation_parametric_rendering::surface_trim::bezier_curve_tessellate::adaptive_tessellate_bezier_curve(curve.clone(), 1e-3);
    if pts.len() < 2 {
      eprintln!(
        "    curve {}/{}: skipped (too few points)",
        i + 1,
        data.curves_3d.len()
      );
      continue;
    }
    eprintln!(
      "    curve {}/{}: {} line points",
      i + 1,
      data.curves_3d.len(),
      pts.len()
    );
    doc.add_curve_mesh(&pts, i, use_line_list);
  }

  let (root, bin) = doc.into_root();
  eprintln!(
    "writing glTF: {} ({} meshes, {} nodes, {}K binary)",
    gltf_path.display(),
    root.meshes.len(),
    root.nodes.len(),
    bin.len() / 1024
  );
  write_glb(gltf_path, &root, &bin)?;

  eprintln!("done.");
  Ok(())
}
