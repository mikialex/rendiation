//! STEP → glTF converter.
//!
//! Reads a STEP file, triangulates every trimmed surface and tessellates
//! 3D edge curves, then exports a single `.glb` file.
//!
//! ```sh
//! cargo run -p rendiation-parametric-rendering --example gltf -- input.stp output.glb
//! ```

use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use gltf_json as json;
use json::validation::{Checked, USize64};
use rendiation_algebra::*;
use rendiation_parametric_rendering::mesh::{
  tessellate_curve, triangulate_trimmed_surface, MeshData, TriangulationConfig,
};
use rendiation_parametric_rendering::step::{
  read_parametric_rendering_data_from_step, StepReadConfig,
};

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

struct GltfDoc {
  bin: Vec<u8>,
  buffer_views: Vec<json::buffer::View>,
  accessors: Vec<json::Accessor>,
  meshes: Vec<json::Mesh>,
  materials: Vec<json::Material>,
  nodes: Vec<json::Node>,
  root_children: Vec<u32>,
  extensions_used: std::collections::BTreeSet<String>,
  mesh_cache: HashMap<usize, u32>,
  curve_mesh_cache: HashMap<usize, u32>,
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
      mesh_cache: HashMap::new(),
      curve_mesh_cache: HashMap::new(),
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

  fn create_surface_mesh(&mut self, mesh: &MeshData, label: &str, surf_idx: usize) {
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
      name: Some(label.to_string()),
      extensions: None,
      extras: Default::default(),
      weights: None,
    });

    let mesh_idx = (self.meshes.len() - 1) as u32;
    self.mesh_cache.insert(surf_idx, mesh_idx);
  }

  fn add_surface_instance(&mut self, surf_idx: usize, matrix: Mat4<f32>, label: &str) {
    let Some(&mesh_idx) = self.mesh_cache.get(&surf_idx) else {
      return; // surface was skipped (empty triangulation)
    };
    let matrix_values = [
      matrix.a1, matrix.a2, matrix.a3, matrix.a4, matrix.b1, matrix.b2, matrix.b3, matrix.b4,
      matrix.c1, matrix.c2, matrix.c3, matrix.c4, matrix.d1, matrix.d2, matrix.d3, matrix.d4,
    ];
    let node_idx = self.nodes.len() as u32;
    self.nodes.push(json::Node {
      mesh: Some(json::Index::new(mesh_idx)),
      matrix: Some(matrix_values),
      name: Some(label.to_string()),
      ..Default::default()
    });
    self.root_children.push(node_idx);
  }

  fn create_curve_mesh(&mut self, points: &[Vec3<f32>], curve_idx: usize, use_line_list: bool) {
    if points.len() < 2 {
      return;
    }
    let pos_acc = self.add_vec3_accessor(points);
    let (line_indices, mode) = if use_line_list {
      // LINE_LIST: each segment is an independent pair (i, i+1)
      let n = points.len();
      let pairs: Vec<u32> = (0..n as u32 - 1).flat_map(|i| [i, i + 1]).collect();
      (pairs, json::mesh::Mode::Lines)
    } else {
      let indices: Vec<u32> = (0..points.len() as u32).collect();
      (indices, json::mesh::Mode::LineStrip)
    };
    let idx_acc = self.add_u32_index_accessor_flat(&line_indices);

    self
      .extensions_used
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
    self.curve_mesh_cache.insert(curve_idx, mesh_idx);
  }

  fn add_curve_instance(&mut self, curve_idx: usize, matrix: Mat4<f32>, label: &str) {
    let Some(&mesh_idx) = self.curve_mesh_cache.get(&curve_idx) else {
      return; // curve was skipped (empty tessellation)
    };
    let matrix_values = [
      matrix.a1, matrix.a2, matrix.a3, matrix.a4, matrix.b1, matrix.b2, matrix.b3, matrix.b4,
      matrix.c1, matrix.c2, matrix.c3, matrix.c4, matrix.d1, matrix.d2, matrix.d3, matrix.d4,
    ];
    let node_idx = self.nodes.len() as u32;
    self.nodes.push(json::Node {
      mesh: Some(json::Index::new(mesh_idx)),
      matrix: Some(matrix_values),
      name: Some(label.to_string()),
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
      extensions_used: self.extensions_used.into_iter().collect::<Vec<_>>(),
      ..Default::default()
    };
    (root, bin)
  }
}

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();
  if args.len() < 3 {
    println!("usage: {} <input.stp> <output.glb>", args[0]);
    std::process::exit(1);
  }

  let step_path = Path::new(&args[1]);
  let gltf_path = Path::new(&args[2]);
  let use_line_list = true;

  println!("reading STEP: {}", step_path.display());
  let raw = fs::read_to_string(step_path)?;
  let step_str = raw;

  let step_config = StepReadConfig::default();
  let result = read_parametric_rendering_data_from_step(&step_str, step_config);
  result.print_errors();
  let data = result.data;

  println!(
    "  {} unique surfaces ({} instances), {} unique 3D curves ({} instances)",
    data.surfaces.len(),
    data.surfaces_instance.len(),
    data.curves_3d.len(),
    data.curves_3d_instance.len()
  );

  let mut tri_config = TriangulationConfig::default();
  tri_config.ignore_surface_trim = false;

  let mut doc = GltfDoc::new();

  // Phase 1: create meshes for unique surfaces (geometry in local coordinates).
  for (surf_idx, trimmed) in data.surfaces.iter().enumerate() {
    println!(
      "  triangulating surface {}/{} [{}]{}",
      surf_idx + 1,
      data.surfaces.len(),
      trimmed.debug_label,
      if !trimmed.is_trimmed() {
        " (untrimmed)"
      } else {
        ""
      }
    );
    let mesh = triangulate_trimmed_surface(trimmed, &tri_config);
    if mesh.indices.is_empty() {
      println!("    skipped (empty triangulation)");
      continue;
    }
    println!(
      "    {} vertices, {} triangles",
      mesh.positions.len(),
      mesh.indices.len()
    );
    doc.create_surface_mesh(&mesh, &trimmed.debug_label, surf_idx);
  }

  // Phase 2: create nodes for surface instances, each with its own matrix.
  for (inst_idx, &(surf_idx, matrix)) in data.surfaces_instance.iter().enumerate() {
    let trimmed = &data.surfaces[surf_idx];
    let label = format!("{}_inst{}", trimmed.debug_label, inst_idx);
    doc.add_surface_instance(surf_idx, matrix, &label);
  }

  // Phase 1: create meshes for unique 3D curves.
  if !data.curves_3d.is_empty() {
    println!(
      "  tessellating {} unique 3D curves...",
      data.curves_3d.len()
    );
  }
  for (curve_idx, curve) in data.curves_3d.iter().enumerate() {
    let pts = tessellate_curve(curve, 1e-3);
    if pts.len() < 2 {
      println!(
        "    curve {}/{}: skipped (too few points)",
        curve_idx + 1,
        data.curves_3d.len()
      );
      continue;
    }
    println!(
      "    curve {}/{}: {} line points",
      curve_idx + 1,
      data.curves_3d.len(),
      pts.len()
    );
    doc.create_curve_mesh(&pts, curve_idx, use_line_list);
  }

  // Phase 2: create nodes for curve instances.
  for (inst_idx, &(curve_idx, matrix)) in data.curves_3d_instance.iter().enumerate() {
    let label = format!("curve_{}_inst{}", curve_idx, inst_idx);
    doc.add_curve_instance(curve_idx, matrix, &label);
  }

  let (root, bin) = doc.into_root();
  println!(
    "writing glTF: {} ({} meshes, {} nodes, {}K binary)",
    gltf_path.display(),
    root.meshes.len(),
    root.nodes.len(),
    bin.len() / 1024
  );
  write_glb(gltf_path, &root, &bin)?;

  println!("done.");
  Ok(())
}
