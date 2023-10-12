use std::path::Path;

use rendiation_algebra::*;
use rendiation_scene_core::{
  AttributeAccessor, AttributeIndexFormat, AttributeSemantic, AttributesMesh,
  IntoIncrementalSignalPtr, MaterialEnum, MeshEnum, ModelEnum, NormalMapping,
  PhysicalSpecularGlossinessMaterial, Scene, SceneExt, SceneModelImpl, SceneTexture2D,
  SceneTexture2DType, StandardModel, Texture2DWithSamplingData,
};
use rendiation_texture::*;
use smallvec::SmallVec;

#[derive(thiserror::Error, Debug)]
pub enum ObjLoadError {
  #[error("Obj load or parse failed: {0}")]
  ObjLoadErr(#[from] tobj::LoadError),
}

pub fn load_obj(
  path: impl AsRef<Path> + std::fmt::Debug,
  scene: &Scene,
) -> Result<(), ObjLoadError> {
  let models = load_obj_content(path, obj_loader_recommended_default_mat)?;
  let node = scene.create_root_child();
  for model in models {
    let model = SceneModelImpl::new(ModelEnum::Standard(model.into_ptr()), node.clone()).into_ptr();
    scene.insert_model(model);
  }
  Ok(())
}

pub fn load_obj_content(
  path: impl AsRef<Path> + std::fmt::Debug,
  create_default_material: impl Fn() -> MaterialEnum,
) -> Result<Vec<StandardModel>, ObjLoadError> {
  let (models, materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;

  let models = models
    .iter()
    .map(|m| {
      let indices = &m.mesh.indices;
      let indices = AttributeAccessor::create_owned(indices.clone(), 4);

      let mut attributes = SmallVec::with_capacity(3);
      attributes.push((
        AttributeSemantic::Positions,
        AttributeAccessor::create_owned(m.mesh.positions.clone(), 3 * 4),
      ));
      let vertices_count = m.mesh.positions.len() / 3;

      if !m.mesh.normals.is_empty() {
        attributes.push((
          AttributeSemantic::Normals,
          AttributeAccessor::create_owned(m.mesh.normals.clone(), 3 * 4),
        ));
      } else {
        // should we make this behavior configurable?
        attributes.push((
          AttributeSemantic::Normals,
          AttributeAccessor::create_owned(
            vec![Vec3::<f32>::new(1., 0., 0.); vertices_count],
            3 * 4,
          ),
        ));
      }
      if !m.mesh.texcoords.is_empty() {
        attributes.push((
          AttributeSemantic::TexCoords(0),
          AttributeAccessor::create_owned(m.mesh.texcoords.clone(), 2 * 4),
        ));
      } else {
        // should we make this behavior configurable?
        attributes.push((
          AttributeSemantic::TexCoords(0),
          AttributeAccessor::create_owned(vec![Vec2::<f32>::new(0., 0.); vertices_count], 2 * 4),
        ));
      }

      // we used GPU_LOAD_OPTIONS, so we can assure only has one index buffer
      let attribute_mesh = AttributesMesh {
        attributes,
        indices: (AttributeIndexFormat::Uint32, indices).into(),
        mode: rendiation_mesh_core::PrimitiveTopology::TriangleList,
        groups: Default::default(),
      };
      let mesh = MeshEnum::AttributesMesh(attribute_mesh.into_ptr());

      let mut material = None;
      if let Some(material_id) = m.mesh.material_id {
        if let Ok(materials) = &materials {
          if let Some(m) = materials.get(material_id) {
            material = into_rff_material(m).into();
          }
        }
      }
      let material = material.unwrap_or(create_default_material());

      StandardModel {
        material,
        mesh,
        group: Default::default(),
        skeleton: None,
      }
    })
    .collect();
  Ok(models)
}

pub fn obj_loader_recommended_default_mat() -> MaterialEnum {
  let mat = PhysicalSpecularGlossinessMaterial::default();
  MaterialEnum::PhysicalSpecularGlossiness(mat.into_ptr())
}

/// convert obj material into scene material, only part of material parameters are supported
fn into_rff_material(m: &tobj::Material) -> MaterialEnum {
  let mut mat = PhysicalSpecularGlossinessMaterial::default();
  if let Some(diffuse) = m.diffuse {
    mat.albedo = Vec3::new(diffuse[0], diffuse[1], diffuse[2]);
  }
  if let Some(specular) = m.specular {
    mat.specular = Vec3::new(specular[0], specular[1], specular[2]);
  }
  if let Some(diffuse_texture) = &m.diffuse_texture {
    mat.albedo_texture = load_texture_sampler_pair(diffuse_texture).into();
  }
  if let Some(specular_texture) = &m.specular_texture {
    mat.specular_texture = load_texture_sampler_pair(specular_texture).into();
  }
  if let Some(normal_texture) = &m.normal_texture {
    mat.normal_texture = load_normal_map(normal_texture).into();
  }
  MaterialEnum::PhysicalSpecularGlossiness(mat.into_ptr())
}

fn load_texture_sampler_pair(path: impl AsRef<Path>) -> Texture2DWithSamplingData {
  Texture2DWithSamplingData {
    texture: load_tex(path),
    sampler: TextureSampler::tri_linear_repeat().into_ptr(),
  }
}

fn load_normal_map(path: impl AsRef<Path>) -> NormalMapping {
  NormalMapping {
    content: load_texture_sampler_pair(path),
    scale: 1.0,
  }
}

// todo texture loader should passed in and config ability freely
fn load_tex(path: impl AsRef<Path>) -> SceneTexture2D {
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
  SceneTexture2DType::GPUBufferImage(tex).into_ptr()
}
