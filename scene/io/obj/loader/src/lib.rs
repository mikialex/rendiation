use std::path::Path;

use database::*;
use rendiation_algebra::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_texture_loader::*;
use smallvec::SmallVec;

#[derive(thiserror::Error, Debug)]
pub enum ObjLoadError {
  #[error("Obj load or parse failed: {0}")]
  ObjLoadErr(#[from] tobj::LoadError),
}

pub fn load_obj(
  path: impl AsRef<Path> + std::fmt::Debug,
  node: EntityHandle<SceneNodeEntity>,
  default_mat: EntityHandle<PbrSGMaterialEntity>,
  writer: &mut SceneWriter,
) -> Result<(), ObjLoadError> {
  let models = load_obj_content(path, default_mat, writer)?;

  for model in models {
    let std_model = model.write(&mut writer.std_model_writer);

    let sm = SceneModelDataView {
      model: std_model,
      scene: writer.scene,
      node,
    };

    sm.write(&mut writer.model_writer);
  }
  Ok(())
}

pub fn load_obj_content(
  path: impl AsRef<Path> + std::fmt::Debug,
  default_mat: EntityHandle<PbrSGMaterialEntity>,
  writer: &mut SceneWriter,
) -> Result<Vec<StandardModelDataView>, ObjLoadError> {
  let (models, materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;

  let models = models
    .iter()
    .map(|m| {
      let attribute_mesh = create_attribute_mesh_from_obj_mesh(&m.mesh);
      let attribute_mesh = writer.write_attribute_mesh(attribute_mesh);

      let mut material = None;
      if let Some(material_id) = m.mesh.material_id {
        if let Ok(materials) = &materials {
          if let Some(m) = materials.get(material_id) {
            material = into_rff_material(m, writer)
              .write(&mut writer.pbr_sg_mat_writer)
              .into();
          }
        }
      }
      let material = material.unwrap_or(default_mat);

      StandardModelDataView {
        material: SceneMaterialDataView::PbrSGMaterial(material),
        mesh: attribute_mesh,
      }
    })
    .collect();
  Ok(models)
}

/// convert obj material into scene material, only part of material parameters are supported
fn into_rff_material(
  m: &tobj::Material,
  writer: &mut SceneWriter,
) -> PhysicalSpecularGlossinessMaterialDataView {
  let mut mat = PhysicalSpecularGlossinessMaterialDataView::default();
  if let Some(diffuse) = m.diffuse {
    mat.albedo = Vec3::new(diffuse[0], diffuse[1], diffuse[2]);
  }
  if let Some(specular) = m.specular {
    mat.specular = Vec3::new(specular[0], specular[1], specular[2]);
  }
  if let Some(diffuse_texture) = &m.diffuse_texture {
    let diffuse_texture = load_tex(diffuse_texture);
    mat.albedo_texture = writer
      .texture_sample_pair_writer()
      .write_tex_with_default_sampler(diffuse_texture)
      .into();
  }
  if let Some(specular_texture) = &m.specular_texture {
    let specular_texture = load_tex(specular_texture);
    mat.specular_texture = writer
      .texture_sample_pair_writer()
      .write_tex_with_default_sampler(specular_texture)
      .into();
  }
  if let Some(normal_texture) = &m.normal_texture {
    let normal_texture = load_tex(normal_texture);
    mat.normal_texture = Some(NormalMappingDataView {
      scale: 1.0,
      content: writer
        .texture_sample_pair_writer()
        .write_tex_with_default_sampler(normal_texture),
    });
  }
  mat
}

fn create_attribute_mesh_from_obj_mesh(mesh: &tobj::Mesh) -> AttributesMesh {
  let indices = &mesh.indices;
  let indices = AttributeAccessor::create_owned(indices.clone(), 4);

  let mut attributes = SmallVec::with_capacity(3);
  attributes.push((
    AttributeSemantic::Positions,
    AttributeAccessor::create_owned(mesh.positions.clone(), 3 * 4),
  ));
  let vertices_count = mesh.positions.len() / 3;

  if !mesh.normals.is_empty() {
    attributes.push((
      AttributeSemantic::Normals,
      AttributeAccessor::create_owned(mesh.normals.clone(), 3 * 4),
    ));
  } else {
    // should we make this behavior configurable?
    attributes.push((
      AttributeSemantic::Normals,
      AttributeAccessor::create_owned(vec![Vec3::<f32>::new(1., 0., 0.); vertices_count], 3 * 4),
    ));
  }
  if !mesh.texcoords.is_empty() {
    attributes.push((
      AttributeSemantic::TexCoords(0),
      AttributeAccessor::create_owned(mesh.texcoords.clone(), 2 * 4),
    ));
  } else {
    // should we make this behavior configurable?
    attributes.push((
      AttributeSemantic::TexCoords(0),
      AttributeAccessor::create_owned(vec![Vec2::<f32>::new(0., 0.); vertices_count], 2 * 4),
    ));
  }

  // we used GPU_LOAD_OPTIONS, so we can assure only has one index buffer
  AttributesMesh {
    attributes,
    indices: (AttributeIndexFormat::Uint32, indices).into(),
    mode: rendiation_mesh_core::PrimitiveTopology::TriangleList,
    groups: Default::default(),
  }
}
