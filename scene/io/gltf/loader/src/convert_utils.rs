use rendiation_algebra::{Mat4, Quat};

pub fn map_sampler(sampler: gltf::texture::Sampler) -> rendiation_texture::TextureSampler {
  rendiation_texture::TextureSampler {
    address_mode_u: map_wrapping(sampler.wrap_s()),
    address_mode_v: map_wrapping(sampler.wrap_t()),
    address_mode_w: rendiation_texture::AddressMode::ClampToEdge,
    mag_filter: sampler
      .mag_filter()
      .map(map_mag_filter)
      .unwrap_or(rendiation_texture::FilterMode::Nearest),
    min_filter: sampler
      .min_filter()
      .map(map_min_filter)
      .unwrap_or(rendiation_texture::FilterMode::Nearest),
    mipmap_filter: sampler
      .min_filter()
      .map(map_min_filter_mipmap)
      .unwrap_or(rendiation_texture::FilterMode::Nearest),
  }
}

pub fn map_wrapping(mode: gltf::texture::WrappingMode) -> rendiation_texture::AddressMode {
  match mode {
    gltf::texture::WrappingMode::ClampToEdge => rendiation_texture::AddressMode::ClampToEdge,
    gltf::texture::WrappingMode::MirroredRepeat => rendiation_texture::AddressMode::MirrorRepeat,
    gltf::texture::WrappingMode::Repeat => rendiation_texture::AddressMode::Repeat,
  }
}

pub fn map_min_filter(min: gltf::texture::MinFilter) -> rendiation_texture::FilterMode {
  match min {
    gltf::texture::MinFilter::Nearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::Linear => rendiation_texture::FilterMode::Linear,
    gltf::texture::MinFilter::NearestMipmapNearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::LinearMipmapNearest => rendiation_texture::FilterMode::Linear,
    gltf::texture::MinFilter::NearestMipmapLinear => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::LinearMipmapLinear => rendiation_texture::FilterMode::Linear,
  }
}

/// https://www.khronos.org/opengl/wiki/Sampler_Object
pub fn map_min_filter_mipmap(min: gltf::texture::MinFilter) -> rendiation_texture::FilterMode {
  match min {
    gltf::texture::MinFilter::Nearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::Linear => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::NearestMipmapNearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::LinearMipmapNearest => rendiation_texture::FilterMode::Nearest,
    gltf::texture::MinFilter::NearestMipmapLinear => rendiation_texture::FilterMode::Linear,
    gltf::texture::MinFilter::LinearMipmapLinear => rendiation_texture::FilterMode::Linear,
  }
}

pub fn map_mag_filter(f: gltf::texture::MagFilter) -> rendiation_texture::FilterMode {
  match f {
    gltf::texture::MagFilter::Nearest => todo!(),
    gltf::texture::MagFilter::Linear => todo!(),
  }
}

pub fn map_draw_mode(mode: gltf::mesh::Mode) -> Option<webgpu::PrimitiveTopology> {
  match mode {
    gltf::mesh::Mode::Points => webgpu::PrimitiveTopology::PointList,
    gltf::mesh::Mode::Lines => webgpu::PrimitiveTopology::LineList,
    gltf::mesh::Mode::LineLoop => return None,
    gltf::mesh::Mode::LineStrip => webgpu::PrimitiveTopology::LineStrip,
    gltf::mesh::Mode::Triangles => webgpu::PrimitiveTopology::TriangleList,
    gltf::mesh::Mode::TriangleStrip => webgpu::PrimitiveTopology::TriangleStrip,
    gltf::mesh::Mode::TriangleFan => return None,
  }
  .into()
}

pub fn map_transform(t: gltf::scene::Transform) -> Mat4<f32> {
  match t {
    gltf::scene::Transform::Matrix { matrix } => {
      Mat4::new_from_colum(matrix[0], matrix[1], matrix[2], matrix[3])
    }
    gltf::scene::Transform::Decomposed {
      translation,
      rotation,
      scale,
    } => Mat4::translate(translation) * Mat4::from(Quat::from(rotation)) * Mat4::scale(scale),
  }
}
