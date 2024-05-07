use std::path::Path;

use database::*;
use rendiation_scene_core::*;
use rendiation_texture_core::*;

pub fn load_texture_sampler_pair<E, C>(
  path: impl AsRef<Path>,
  writer: &mut EntityWriter<E>,
  tex_writer: &mut EntityWriter<SceneTexture2dEntity>,
  sampler_writer: &mut EntityWriter<SceneSamplerEntity>,
) where
  C: TextureWithSamplingForeignKeys,
  C: EntityAssociateSemantic<Entity = E>,
  E: EntitySemantic,
{
  let sampler = sampler_writer
    .component_value_writer::<SceneSamplerInfo>(TextureSampler::tri_linear_repeat())
    .new_entity();

  Texture2DWithSamplingDataView {
    texture: load_tex(path, tex_writer),
    sampler,
  }
  .write::<C, E>(writer)
}

// todo texture loader should passed in and config ability freely
pub fn load_tex(
  path: impl AsRef<Path>,
  writer: &mut EntityWriter<SceneTexture2dEntity>,
) -> EntityHandle<SceneTexture2dEntity> {
  use image::io::Reader as ImageReader;
  let img = ImageReader::open(path).unwrap().decode().unwrap();
  let tex = match img {
    image::DynamicImage::ImageRgba8(img) => {
      let size = img.size();
      let format = TextureFormat::Rgba8UnormSrgb;
      let data = img.into_raw();
      GPUBufferImage { data, format, size }
    }
    image::DynamicImage::ImageRgb8(img) => {
      let size = img.size();
      let format = TextureFormat::Rgba8UnormSrgb;
      let data = create_padding_buffer(img.as_raw(), 3, &[255]);
      GPUBufferImage { data, format, size }
    }
    _ => panic!("unsupported texture type"),
  };
  let tex = ExternalRefPtr::new(tex);

  writer
    .component_value_writer::<SceneTexture2dEntityDirectContent>(tex.into())
    .new_entity()
}
