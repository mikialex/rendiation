use rendiation_renderable_mesh::PrimitiveTopology;
use rendiation_scene_core::AlphaMode;

pub fn map_draw_mode(mode: PrimitiveTopology) -> gltf_json::mesh::Mode {
  match mode {
    PrimitiveTopology::PointList => gltf_json::mesh::Mode::Points,
    PrimitiveTopology::LineList => gltf_json::mesh::Mode::Lines,
    PrimitiveTopology::LineStrip => gltf_json::mesh::Mode::LineStrip,
    PrimitiveTopology::TriangleList => gltf_json::mesh::Mode::Triangles,
    PrimitiveTopology::TriangleStrip => gltf_json::mesh::Mode::TriangleStrip,
  }
}

pub fn map_wrapping(mode: rendiation_texture::AddressMode) -> gltf_json::texture::WrappingMode {
  match mode {
    rendiation_texture::AddressMode::ClampToEdge => gltf_json::texture::WrappingMode::ClampToEdge,
    rendiation_texture::AddressMode::MirrorRepeat => {
      gltf_json::texture::WrappingMode::MirroredRepeat
    }
    rendiation_texture::AddressMode::Repeat => gltf_json::texture::WrappingMode::Repeat,
  }
}

pub fn map_alpha_mode(alpha_mode: AlphaMode) -> gltf_json::material::AlphaMode {
  match alpha_mode {
    AlphaMode::Opaque => gltf_json::material::AlphaMode::Opaque,
    AlphaMode::Mask => gltf_json::material::AlphaMode::Mask,
    AlphaMode::Blend => gltf_json::material::AlphaMode::Blend,
  }
}
