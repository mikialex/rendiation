use std::path::Path;

use database::*;
use rendiation_algebra::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_io_texture_loader::*;
use smallvec::SmallVec;

#[derive(thiserror::Error, Debug)]
pub enum ObjLoadError {
  #[error("Obj load or parse failed: {0}")]
  ObjLoadErr(#[from] tobj::LoadError),
}

pub fn load_obj(
  path: impl AsRef<Path> + std::fmt::Debug,
  node: EntityHandle<SceneNodeEntity>,
  scene: EntityHandle<SceneEntity>,
  default_mat: EntityHandle<PbrSGMaterialEntity>,
) -> Result<(), ObjLoadError> {
  let models = load_obj_content(path, default_mat)?;

  let mut std_model_w = global_entity_of().entity_writer();
  let mut sm_w = global_entity_of().entity_writer();

  for model in models {
    let std_model = model.write(&mut std_model_w);

    let sm = SceneModelDataModel {
      model: std_model,
      scene,
      node,
    };

    sm.write(&mut sm_w);
  }
  Ok(())
}

pub fn load_obj_content(
  path: impl AsRef<Path> + std::fmt::Debug,
  default_mat: EntityHandle<PbrSGMaterialEntity>,
) -> Result<Vec<StandardModelDataView>, ObjLoadError> {
  let (models, materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;

  let mut mesh_writer = AttributesMesh::create_writer();
  let mut tex_writer = TexSamplerWriter::default();
  let mut material_writer = global_entity_of().entity_writer();

  let models = models
    .iter()
    .map(|m| {
      let attribute_mesh = create_attribute_mesh_from_obj_mesh(&m.mesh);
      let attribute_mesh = attribute_mesh.write(&mut mesh_writer);

      let mut material = None;
      if let Some(material_id) = m.mesh.material_id {
        if let Ok(materials) = &materials {
          if let Some(m) = materials.get(material_id) {
            material = into_rff_material(m, &mut tex_writer)
              .write(&mut material_writer)
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

// pub fn obj_loader_recommended_default_mat() -> AllocIdx<FlatMaterialEntity> {
//   let mat = PhysicalSpecularGlossinessMaterial::default();
//   MaterialEnum::PhysicalSpecularGlossiness(mat.into_ptr())
// }

/// convert obj material into scene material, only part of material parameters are supported
fn into_rff_material(
  m: &tobj::Material,
  tex_writer: &mut TexSamplerWriter,
) -> PhysicalSpecularGlossinessMaterialDataView {
  let mut mat = PhysicalSpecularGlossinessMaterialDataView::default();
  if let Some(diffuse) = m.diffuse {
    mat.albedo = Vec3::new(diffuse[0], diffuse[1], diffuse[2]);
  }
  if let Some(specular) = m.specular {
    mat.specular = Vec3::new(specular[0], specular[1], specular[2]);
  }
  if let Some(diffuse_texture) = &m.diffuse_texture {
    mat.albedo_texture = load_texture_sampler_pair(diffuse_texture, tex_writer).into();
  }
  if let Some(specular_texture) = &m.specular_texture {
    mat.specular_texture = load_texture_sampler_pair(specular_texture, tex_writer).into();
  }
  if let Some(normal_texture) = &m.normal_texture {
    mat.normal_texture = Some(NormalMappingDataView {
      scale: 1.0,
      content: load_texture_sampler_pair(normal_texture, tex_writer),
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
