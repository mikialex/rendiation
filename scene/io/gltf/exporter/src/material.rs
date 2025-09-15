use crate::*;

pub fn build_material(
  materials: &mut Resource<SceneMaterialDataView, gltf_json::Material>,
  reader: &SceneReader,
  m: &SceneMaterialDataView,
  textures: &Resource<(EntityHandle<SceneTexture2dEntity>, TextureSampler), gltf_json::Texture>,
) -> Option<gltf_json::Index<gltf_json::Material>> {
  match m {
    SceneMaterialDataView::UnlitMaterial(material) => materials.append(*m, {
      let material = reader.read_unlit_material(*material);
      gltf_json::Material {
        alpha_cutoff: gltf_json::material::AlphaCutoff(material.alpha.alpha_cutoff).into(),
        alpha_mode: gltf_json::validation::Checked::Valid(map_alpha_mode(
          material.alpha.alpha_mode,
        )),
        pbr_metallic_roughness: gltf_json::material::PbrMetallicRoughness {
          base_color_factor: gltf_json::material::PbrBaseColorFactor([
            material.color.x,
            material.color.y,
            material.color.z,
            1.,
          ]),
          base_color_texture: material
            .color_alpha_tex
            .as_ref()
            .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
          ..Default::default()
        },
        extensions: Some(gltf_json::extensions::material::Material {
          unlit: Some(gltf_json::extensions::material::Unlit {}),
          ..Default::default()
        }),
        ..Default::default()
      }
    }),
    SceneMaterialDataView::PbrSGMaterial(material) => materials.append(*m, {
      let material = reader.read_pbr_sg_material(*material);
      gltf_json::Material {
        alpha_cutoff: gltf_json::material::AlphaCutoff(material.alpha.alpha_cutoff).into(),
        alpha_mode: gltf_json::validation::Checked::Valid(map_alpha_mode(
          material.alpha.alpha_mode,
        )),
        extensions: Some(gltf_json::extensions::material::Material {
          pbr_specular_glossiness: Some(gltf_json::extensions::material::PbrSpecularGlossiness {
            diffuse_factor: gltf_json::extensions::material::PbrDiffuseFactor(
              material.albedo.expand_with_one().into(),
            ),
            diffuse_texture: material
              .albedo_texture
              .as_ref()
              .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
            specular_factor: gltf_json::extensions::material::PbrSpecularFactor(
              material.specular.into(),
            ),
            glossiness_factor: gltf_json::material::StrengthFactor(material.glossiness),
            specular_glossiness_texture: material
              .specular_glossiness_texture
              .as_ref()
              .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
            extras: Default::default(),
          }),
          ..Default::default()
        }),
        ..Default::default()
      }
    }),
    SceneMaterialDataView::PbrMRMaterial(material) => materials.append(*m, {
      let material = reader.read_pbr_mr_material(*material);
      gltf_json::Material {
        alpha_cutoff: gltf_json::material::AlphaCutoff(material.alpha.alpha_cutoff).into(),
        alpha_mode: gltf_json::validation::Checked::Valid(map_alpha_mode(
          material.alpha.alpha_mode,
        )),
        pbr_metallic_roughness: gltf_json::material::PbrMetallicRoughness {
          base_color_factor: gltf_json::material::PbrBaseColorFactor(
            material.base_color.expand_with_one().into(),
          ),
          base_color_texture: material
            .base_color_texture
            .as_ref()
            .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
          metallic_factor: gltf_json::material::StrengthFactor(material.metallic),
          roughness_factor: gltf_json::material::StrengthFactor(material.roughness),
          metallic_roughness_texture: material
            .metallic_roughness_texture
            .as_ref()
            .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
          ..Default::default()
        },
        normal_texture: material.normal_texture.as_ref().and_then(|t| {
          gltf_json::material::NormalTexture {
            index: {
              let sampler_content = reader.read_sampler(t.content.sampler);
              textures.get(&(t.content.texture, sampler_content))
            },
            scale: t.scale,
            tex_coord: 0,
            extensions: Default::default(),
            extras: Default::default(),
          }
          .into()
        }),
        emissive_texture: material
          .emissive_texture
          .as_ref()
          .and_then(|t| get_texture2d_info(t, 0, reader, textures)),
        emissive_factor: gltf_json::material::EmissiveFactor(material.emissive.into()),
        ..Default::default()
      }
    }),
    _ => return None,
  }
  .into()
}

fn get_texture2d_info(
  ts: &Texture2DWithSamplingDataView,
  tex_coord: usize,
  reader: &SceneReader,
  textures: &Resource<(EntityHandle<SceneTexture2dEntity>, TextureSampler), gltf_json::Texture>,
) -> Option<gltf_json::texture::Info> {
  let sampler_content = reader.read_sampler(ts.sampler);
  gltf_json::texture::Info {
    index: textures.get(&(ts.texture, sampler_content)),
    tex_coord: tex_coord as u32,
    extensions: Default::default(),
    extras: Default::default(),
  }
  .into()
}

pub fn build_texture2d(
  images: &mut Resource<EntityHandle<SceneTexture2dEntity>, gltf_json::Image>,
  samplers: &mut Resource<TextureSampler, gltf_json::texture::Sampler>,
  textures: &mut Resource<(EntityHandle<SceneTexture2dEntity>, TextureSampler), gltf_json::Texture>,
  data_writer: Option<&mut BufferResourceInliner>,
  reader: &SceneReader,
  ts: &Texture2DWithSamplingDataView,
) -> gltf_json::Index<gltf_json::Texture> {
  let source = images.append(ts.texture, {
    let texture = reader.read_texture(ts.texture);

    let mut png_buffer = Vec::new(); // todo avoid extra copy
    rendiation_texture_exporter::write_gpu_buffer_image_as_png(&mut png_buffer, &texture);

    let mut image = gltf_json::Image {
      buffer_view: Default::default(),
      mime_type: Some(gltf_json::image::MimeType("image/png".to_string())),
      name: Default::default(),
      uri: Default::default(),
      extensions: Default::default(),
      extras: Default::default(),
    };

    if let Some(data_writer) = data_writer {
      image.buffer_view = data_writer
        .collect_inline_packed_view_buffer(&png_buffer)
        .into();
    }

    image
  });

  let sampler_content = reader.read_sampler(ts.sampler);
  let sampler = samplers.get_or_insert_with(sampler_content, || map_sampler(sampler_content, true));

  textures.get_or_insert_with((ts.texture, sampler_content), || gltf_json::Texture {
    name: Default::default(),
    sampler: sampler.into(),
    source,
    extensions: Default::default(),
    extras: Default::default(),
  })
}
