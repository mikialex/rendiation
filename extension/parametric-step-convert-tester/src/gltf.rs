use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::Path;

use gltf_json as json;
use json::validation::{Checked, USize64};
use rendiation_algebra::*;
use rendiation_parametric_rendering::mesh::MeshData;

pub fn color_for_index(idx: usize) -> [f32; 4] {
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

pub struct GltfDoc {
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
  pub fn new() -> Self {
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

  fn make_accessor(
    &mut self,
    view_idx: json::Index<json::buffer::View>,
    count: u32,
    comp_ty: json::accessor::ComponentType,
    ty: json::accessor::Type,
    mins: Option<Vec<f32>>,
    maxs: Option<Vec<f32>>,
  ) -> json::Index<json::Accessor> {
    let idx = self.accessors.len() as u32;
    self.accessors.push(json::Accessor {
      buffer_view: Some(view_idx),
      byte_offset: Some(USize64(0)),
      count: USize64(count as u64),
      component_type: Checked::Valid(json::accessor::GenericComponentType(comp_ty)),
      type_: Checked::Valid(ty),
      min: mins.map(|v| json::Value::Array(v.into_iter().map(|f| f.into()).collect())),
      max: maxs.map(|v| json::Value::Array(v.into_iter().map(|f| f.into()).collect())),
      name: None,
      normalized: false,
      sparse: None,
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
    self.make_accessor(
      view_idx,
      count,
      json::accessor::ComponentType::F32,
      json::accessor::Type::Vec3,
      Some(vec![xmin, ymin, zmin]),
      Some(vec![xmax, ymax, zmax]),
    )
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
    self.make_accessor(
      view_idx,
      count,
      json::accessor::ComponentType::F32,
      json::accessor::Type::Vec2,
      Some(vec![xmin, ymin]),
      Some(vec![xmax, ymax]),
    )
  }

  fn add_u32_index_accessor(&mut self, indices: &[[u32; 3]]) -> json::Index<json::Accessor> {
    let flat: Vec<u32> = indices.iter().flat_map(|t| [t[0], t[1], t[2]]).collect();
    let count = flat.len() as u32;
    let bytes = to_u32_bytes(&flat);
    let (off, len) = self.push_bytes(&bytes);
    let view_idx = self.make_arr_view(off, len, json::buffer::Target::ElementArrayBuffer);
    self.make_accessor(
      view_idx,
      count,
      json::accessor::ComponentType::U32,
      json::accessor::Type::Scalar,
      None,
      None,
    )
  }

  fn add_u32_index_accessor_flat(&mut self, indices: &[u32]) -> json::Index<json::Accessor> {
    let count = indices.len() as u32;
    let bytes = to_u32_bytes(indices);
    let (off, len) = self.push_bytes(&bytes);
    let view_idx = self.make_arr_view(off, len, json::buffer::Target::ElementArrayBuffer);
    self.make_accessor(
      view_idx,
      count,
      json::accessor::ComponentType::U32,
      json::accessor::Type::Scalar,
      None,
      None,
    )
  }

  pub fn create_surface_mesh(&mut self, mesh: &MeshData, _label: &str, surf_idx: usize) {
    let pos_acc = self.add_vec3_accessor(&mesh.positions);
    let nrm_acc = self.add_vec3_accessor(&mesh.normals);
    let uv_acc = self.add_vec2_accessor(&mesh.uvs);
    let idx_acc = if !mesh.indices.is_empty() {
      Some(self.add_u32_index_accessor(&mesh.indices))
    } else {
      None
    };
    let mat_idx = self.materials.len() as u32;
    self.materials.push(json::Material {
      name: None,
      extensions: None,
      extras: Default::default(),
      pbr_metallic_roughness: json::material::PbrMetallicRoughness {
        base_color_factor: json::material::PbrBaseColorFactor(color_for_index(mat_idx as usize)),
        metallic_factor: json::material::StrengthFactor(0.0),
        roughness_factor: json::material::StrengthFactor(0.9),
        base_color_texture: None,
        metallic_roughness_texture: None,
        extensions: None,
        extras: Default::default(),
      },
      normal_texture: None,
      occlusion_texture: None,
      emissive_texture: None,
      emissive_factor: json::material::EmissiveFactor([0.0, 0.0, 0.0]),
      alpha_mode: Checked::Valid(json::material::AlphaMode::Opaque),
      alpha_cutoff: Some(json::material::AlphaCutoff(0.5)),
      double_sided: false,
    });
    let mut attrs = BTreeMap::new();
    attrs.insert(Checked::Valid(json::mesh::Semantic::Positions), pos_acc);
    attrs.insert(Checked::Valid(json::mesh::Semantic::Normals), nrm_acc);
    attrs.insert(Checked::Valid(json::mesh::Semantic::TexCoords(0)), uv_acc);
    let prim = json::mesh::Primitive {
      attributes: attrs,
      indices: idx_acc,
      material: Some(json::Index::new(mat_idx)),
      mode: Checked::Valid(json::mesh::Mode::Triangles),
      targets: None,
      extensions: None,
      extras: Default::default(),
    };
    self.meshes.push(json::Mesh {
      primitives: vec![prim],
      weights: None,
      name: None,
      extensions: None,
      extras: Default::default(),
    });
    let mesh_idx = (self.meshes.len() - 1) as u32;
    self.mesh_cache.insert(surf_idx, mesh_idx);
  }

  pub fn add_surface_instance(&mut self, surf_idx: usize, matrix: Mat4<f32>, _label: &str) {
    let Some(&mesh_idx) = self.mesh_cache.get(&surf_idx) else {
      return;
    };
    let node_idx = self.nodes.len() as u32;
    self.nodes.push(json::Node {
      mesh: Some(json::Index::new(mesh_idx)),
      matrix: Some([
        matrix.a1, matrix.a2, matrix.a3, matrix.a4, matrix.b1, matrix.b2, matrix.b3, matrix.b4,
        matrix.c1, matrix.c2, matrix.c3, matrix.c4, matrix.d1, matrix.d2, matrix.d3, matrix.d4,
      ]),
      name: None,
      camera: None,
      skin: None,
      translation: None,
      rotation: None,
      scale: None,
      weights: None,
      children: None,
      extensions: None,
      extras: Default::default(),
    });
    self.root_children.push(node_idx);
  }

  pub fn create_curve_mesh(&mut self, points: &[Vec3<f32>], curve_idx: usize, use_line_list: bool) {
    if points.len() < 2 {
      return;
    }
    let pos_acc = self.add_vec3_accessor(points);
    let (line_indices, mode): (Vec<u32>, _) = if use_line_list {
      let n = points.len();
      (
        (0..n as u32 - 1)
          .flat_map(|i| [i, i + 1])
          .collect::<Vec<_>>(),
        json::mesh::Mode::Lines,
      )
    } else {
      (
        (0..points.len() as u32).collect(),
        json::mesh::Mode::LineStrip,
      )
    };
    let idx_acc = self.add_u32_index_accessor_flat(&line_indices);
    self
      .extensions_used
      .insert("KHR_materials_unlit".to_string());
    let mat_idx = self.materials.len() as u32;
    self.materials.push(json::Material {
      name: None,
      extensions: Some(json::extensions::material::Material {
        unlit: Some(json::extensions::material::Unlit {}),
        ..Default::default()
      }),
      extras: Default::default(),
      pbr_metallic_roughness: json::material::PbrMetallicRoughness {
        base_color_factor: json::material::PbrBaseColorFactor([0.0, 0.0, 0.0, 1.0]),
        metallic_factor: json::material::StrengthFactor(0.0),
        roughness_factor: json::material::StrengthFactor(0.9),
        base_color_texture: None,
        metallic_roughness_texture: None,
        extensions: None,
        extras: Default::default(),
      },
      normal_texture: None,
      occlusion_texture: None,
      emissive_texture: None,
      emissive_factor: json::material::EmissiveFactor([0.0, 0.0, 0.0]),
      alpha_mode: Checked::Valid(json::material::AlphaMode::Opaque),
      alpha_cutoff: Some(json::material::AlphaCutoff(0.5)),
      double_sided: false,
    });
    let mut attrs = BTreeMap::new();
    attrs.insert(Checked::Valid(json::mesh::Semantic::Positions), pos_acc);
    let prim = json::mesh::Primitive {
      attributes: attrs,
      indices: Some(idx_acc),
      material: Some(json::Index::new(mat_idx)),
      mode: Checked::Valid(mode),
      targets: None,
      extensions: None,
      extras: Default::default(),
    };
    self.meshes.push(json::Mesh {
      primitives: vec![prim],
      weights: None,
      name: None,
      extensions: None,
      extras: Default::default(),
    });
    self
      .curve_mesh_cache
      .insert(curve_idx, (self.meshes.len() - 1) as u32);
  }

  pub fn add_curve_instance(&mut self, curve_idx: usize, matrix: Mat4<f32>, _label: &str) {
    let Some(&mesh_idx) = self.curve_mesh_cache.get(&curve_idx) else {
      return;
    };
    let node_idx = self.nodes.len() as u32;
    self.nodes.push(json::Node {
      mesh: Some(json::Index::new(mesh_idx)),
      matrix: Some([
        matrix.a1, matrix.a2, matrix.a3, matrix.a4, matrix.b1, matrix.b2, matrix.b3, matrix.b4,
        matrix.c1, matrix.c2, matrix.c3, matrix.c4, matrix.d1, matrix.d2, matrix.d3, matrix.d4,
      ]),
      name: None,
      camera: None,
      skin: None,
      translation: None,
      rotation: None,
      scale: None,
      weights: None,
      children: None,
      extensions: None,
      extras: Default::default(),
    });
    self.root_children.push(node_idx);
  }

  pub fn into_root(self) -> (json::Root, Vec<u8>) {
    let root = json::Root {
      accessors: self.accessors,
      asset: json::Asset {
        copyright: None,
        generator: None,
        version: String::from("2.0"),
        min_version: None,
        extensions: None,
        extras: Default::default(),
      },
      buffers: vec![json::Buffer {
        byte_length: USize64(self.bin.len() as u64),
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
      extensions_used: self.extensions_used.into_iter().collect(),
      extensions_required: vec![],
      images: vec![],
      textures: vec![],
      samplers: vec![],
      cameras: vec![],
      skins: vec![],
      animations: vec![],
      extensions: None,
      extras: Default::default(),
    };
    (root, self.bin)
  }
}

pub fn write_glb(
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
  out.write_all(&0x4654_6C67_u32.to_le_bytes())?;
  out.write_all(&2u32.to_le_bytes())?;
  out.write_all(&total_len.to_le_bytes())?;
  out.write_all(&(json_padded.len() as u32).to_le_bytes())?;
  out.write_all(&0x4E4F_534A_u32.to_le_bytes())?;
  out.write_all(&json_padded)?;
  if !bin.is_empty() {
    out.write_all(&(bin.len() as u32).to_le_bytes())?;
    out.write_all(&0x004E_4942_u32.to_le_bytes())?;
    out.write_all(bin)?;
  }
  std::fs::write(path, out)?;
  Ok(())
}
