use std::path::Path;

use rendiation_algebra::Vec3;
use rendiation_scene_core::{
  AttributeAccessor, AttributeIndexFormat, AttributesMesh, NormalMapping,
  PhysicalSpecularGlossinessMaterial, SceneMaterialType, SceneMeshType, SceneTexture2D,
  StandardModel, Texture2DWithSamplingData,
};

pub enum ObjLoadError {
  LoadErr(tobj::LoadError),
}

pub fn load_obj(
  path: impl AsRef<Path>,
  create_default_material: impl Fn() -> SceneMaterialType,
) -> Result<Vec<StandardModel>, ObjLoadError> {
  let cornell_box = tobj::load_obj("obj/cornell_box.obj", &tobj::GPU_LOAD_OPTIONS);
  assert!(cornell_box.is_ok());

  let (models, materials) = cornell_box.expect("Failed to load OBJ file");

  // Materials might report a separate loading error if the MTL file wasn't found.
  // If you don't need the materials, you can generate a default here and use that
  // instead.
  let materials = materials.expect("Failed to load MTL file");

  println!("# of models: {}", models.len());
  println!("# of materials: {}", materials.len());

  models
    .iter()
    .map(|m| {
      let positions = m.mesh.positions;
      // AttributeAccessor::create_owned(buffer, first.item_size).into()
      let indices = m.mesh.indices;
      let indices = AttributeAccessor::create_owned(indices, 1);

      // we used GPU_LOAD_OPTIONS, so we can assure only has one index buffer
      let attribute_mesh = AttributesMesh {
        attributes: todo!(),
        indices: (AttributeIndexFormat::Uint32, indices).into(),
        mode: rendiation_renderable_mesh::PrimitiveTopology::TriangleList,
        groups: Default::default(),
      };
      let mesh = SceneMeshType::AttributesMesh(attribute_mesh.into());

      StandardModel {
        material: todo!(),
        mesh,
        group: Default::default(),
        skeleton: None,
      }
    })
    .collect()
}

/// convert obj material into scene material, only part of material parameters are supported
fn into_rff_material(m: tobj::Material) -> PhysicalSpecularGlossinessMaterial {
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
  mat
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
