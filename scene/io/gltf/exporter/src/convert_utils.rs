use rendiation_mesh_core::PrimitiveTopology;
use rendiation_scene_core::{AlphaMode, AttributeSemantic};
use rendiation_texture::{AddressMode, FilterMode, TextureSampler};

pub fn map_draw_mode(mode: PrimitiveTopology) -> gltf_json::mesh::Mode {
  match mode {
    PrimitiveTopology::PointList => gltf_json::mesh::Mode::Points,
    PrimitiveTopology::LineList => gltf_json::mesh::Mode::Lines,
    PrimitiveTopology::LineStrip => gltf_json::mesh::Mode::LineStrip,
    PrimitiveTopology::TriangleList => gltf_json::mesh::Mode::Triangles,
    PrimitiveTopology::TriangleStrip => gltf_json::mesh::Mode::TriangleStrip,
  }
}

pub fn map_wrapping(mode: AddressMode) -> gltf_json::texture::WrappingMode {
  match mode {
    AddressMode::ClampToEdge => gltf_json::texture::WrappingMode::ClampToEdge,
    AddressMode::MirrorRepeat => gltf_json::texture::WrappingMode::MirroredRepeat,
    AddressMode::Repeat => gltf_json::texture::WrappingMode::Repeat,
  }
}

pub fn map_sampler(
  sampler: TextureSampler,
  assume_contains_mipmap: bool,
) -> gltf_json::texture::Sampler {
  let mag_filter = match sampler.mag_filter {
    FilterMode::Nearest => gltf_json::texture::MagFilter::Nearest,
    FilterMode::Linear => gltf_json::texture::MagFilter::Linear,
  };

  #[rustfmt::skip]
  let min_filter = match (sampler.min_filter, sampler.mipmap_filter, assume_contains_mipmap) {
    (FilterMode::Nearest, FilterMode::Nearest, false) => gltf_json::texture::MinFilter::Nearest,
    (FilterMode::Linear, FilterMode::Nearest, false) => gltf_json::texture::MinFilter::Linear,
    (FilterMode::Nearest, FilterMode::Nearest, true) => gltf_json::texture::MinFilter::NearestMipmapNearest,
    (FilterMode::Linear, FilterMode::Nearest, true) => gltf_json::texture::MinFilter::LinearMipmapNearest,
    
    (FilterMode::Nearest, FilterMode::Linear, false) => gltf_json::texture::MinFilter::NearestMipmapNearest, // impossible and fallback
    (FilterMode::Linear, FilterMode::Linear, false) => gltf_json::texture::MinFilter::LinearMipmapNearest, // impossible and fallback
    (FilterMode::Nearest, FilterMode::Linear, true) => gltf_json::texture::MinFilter::NearestMipmapLinear,
    (FilterMode::Linear, FilterMode::Linear, true) => gltf_json::texture::MinFilter::LinearMipmapLinear,
  };

  gltf_json::texture::Sampler {
    mag_filter: gltf_json::validation::Checked::Valid(mag_filter).into(),
    min_filter: gltf_json::validation::Checked::Valid(min_filter).into(),
    wrap_s: gltf_json::validation::Checked::Valid(map_wrapping(sampler.address_mode_u)),
    wrap_t: gltf_json::validation::Checked::Valid(map_wrapping(sampler.address_mode_v)),
    ..Default::default()
  }
}

pub fn map_alpha_mode(alpha_mode: AlphaMode) -> gltf_json::material::AlphaMode {
  match alpha_mode {
    AlphaMode::Opaque => gltf_json::material::AlphaMode::Opaque,
    AlphaMode::Mask => gltf_json::material::AlphaMode::Mask,
    AlphaMode::Blend => gltf_json::material::AlphaMode::Blend,
  }
}

#[rustfmt::skip]
pub fn map_semantic_att(att: &AttributeSemantic) -> Option<(gltf_json::mesh::Semantic, gltf_json::accessor::ComponentType, gltf_json::accessor::Type)> {
  match att {
    AttributeSemantic::Positions => (gltf_json::mesh::Semantic::Positions, gltf_json::accessor::ComponentType::F32, gltf_json::accessor::Type::Vec3),
    AttributeSemantic::Normals => (gltf_json::mesh::Semantic::Normals, gltf_json::accessor::ComponentType::F32, gltf_json::accessor::Type::Vec3),
    AttributeSemantic::Tangents => (gltf_json::mesh::Semantic::Tangents, gltf_json::accessor::ComponentType::F32, gltf_json::accessor::Type::Vec3),
    AttributeSemantic::Colors(v) => (gltf_json::mesh::Semantic::Colors(*v), gltf_json::accessor::ComponentType::F32, gltf_json::accessor::Type::Vec3), // todo check att data
    AttributeSemantic::TexCoords(v) => (gltf_json::mesh::Semantic::TexCoords(*v), gltf_json::accessor::ComponentType::F32, gltf_json::accessor::Type::Vec2),
    AttributeSemantic::Joints(v) => (gltf_json::mesh::Semantic::Joints(*v), gltf_json::accessor::ComponentType::F32, gltf_json::accessor::Type::Scalar), // todo check spec
    AttributeSemantic::Weights(v) => (gltf_json::mesh::Semantic::Weights(*v), gltf_json::accessor::ComponentType::F32, gltf_json::accessor::Type::Vec4), // todo check spec
    AttributeSemantic::Foreign(_) => return None, 
  }.into()
}
