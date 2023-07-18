use std::path::Path;

use rendiation_algebra::Vec3;
use rendiation_scene_core::{
  AttributeAccessor, AttributeIndexFormat, AttributeSemantic, AttributesMesh, IntoSceneItemRef,
  NormalMapping, PhysicalSpecularGlossinessMaterial, SceneMaterialType, SceneMeshType,
  SceneTexture2D, StandardModel, Texture2DWithSamplingData,
};

#[derive(thiserror::Error, Debug)]
pub enum ObjLoadError {
  #[error("Obj load or parse failed: {0}")]
  LoadErr(#[from] tobj::LoadError),
}

pub fn load_obj(
  path: impl AsRef<Path>,
  create_default_material: impl Fn() -> SceneMaterialType,
) -> Result<Vec<StandardModel>, ObjLoadError> {
  let (models, materials) = tobj::load_obj("obj/cornell_box.obj", &tobj::GPU_LOAD_OPTIONS)?;

  let models = models
    .iter()
    .map(|m| {
      let indices = &m.mesh.indices;
      let indices = AttributeAccessor::create_owned(indices.clone(), 1);

      let mut attributes = Vec::with_capacity(3);
      attributes.push((
        AttributeSemantic::Positions,
        AttributeAccessor::create_owned(m.mesh.positions.clone(), 1),
      ));
      let vertices_count = m.mesh.positions.len() / 3;

      if !m.mesh.normals.is_empty() {
        attributes.push((
          AttributeSemantic::Normals,
          AttributeAccessor::create_owned(m.mesh.positions.clone(), 1),
        ));
      } else {
        // should we make this behavior configurable?
        attributes.push((
          AttributeSemantic::Normals,
          AttributeAccessor::create_owned(vec![0.; vertices_count * 3], 1),
        ));
      }
      if !m.mesh.texcoords.is_empty() {
        attributes.push((
          AttributeSemantic::TexCoords(0),
          AttributeAccessor::create_owned(m.mesh.texcoords.clone(), 1),
        ));
      } else {
        // should we make this behavior configurable?
        attributes.push((
          AttributeSemantic::TexCoords(0),
          AttributeAccessor::create_owned(vec![0.; vertices_count * 2], 1),
        ));
      }

      // we used GPU_LOAD_OPTIONS, so we can assure only has one index buffer
      let attribute_mesh = AttributesMesh {
        attributes,
        indices: (AttributeIndexFormat::Uint32, indices).into(),
        mode: rendiation_renderable_mesh::PrimitiveTopology::TriangleList,
        groups: Default::default(),
      };
      let mesh = SceneMeshType::AttributesMesh(attribute_mesh.into_ref());

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

/// convert obj material into scene material, only part of material parameters are supported
fn into_rff_material(m: &tobj::Material) -> SceneMaterialType {
  let mut mat = PhysicalSpecularGlossinessMaterial::default();
  if let Some(diffuse) = m.diffuse {
    mat.albedo = Vec3::new(diffuse[0], diffuse[1], diffuse[2]);
  }
  if let Some(specular) = m.specular {
    mat.specular = Vec3::new(specular[0], specular[1], specular[2]);
  }
  if let Some(diffuse_texture) = &m.diffuse_texture {
    mat.albedo_texture = load_texture_sampler_pair(&diffuse_texture).into();
  }
  if let Some(specular_texture) = &m.specular_texture {
    mat.specular_texture = load_texture_sampler_pair(&specular_texture).into();
  }
  if let Some(normal_texture) = &m.normal_texture {
    mat.normal_texture = load_normal_map(&normal_texture).into();
  }
  SceneMaterialType::PhysicalSpecularGlossiness(mat.into_ref())
}

fn load_texture_sampler_pair(path: impl AsRef<Path>) -> Texture2DWithSamplingData {
  todo!()
}

fn load_normal_map(path: impl AsRef<Path>) -> NormalMapping {
  todo!()
}

fn load_texture(path: impl AsRef<Path>) -> SceneTexture2D {
  todo!()
}
