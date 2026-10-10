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
  target_scene: EntityHandle<SceneEntity>,
  default_mat: EntityHandle<PbrSGMaterialEntity>,
  writer: &mut SceneWriter,
) -> Result<(), ObjLoadError> {
  let models = load_obj_content(path, default_mat, writer)?;

  for model in models {
    let std_model = model.write(&mut writer.std_model_writer);

    let sm = SceneModelDataView {
      model: std_model,
      scene: target_scene,
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
      let attribute_mesh = writer.write_attribute_mesh(attribute_mesh).mesh;

      let mut material = None;
      if let Some(material_id) = m.mesh.material_id
        && let Ok(materials) = &materials
        && let Some(m) = materials.get(material_id)
      {
        material = into_rff_material(m, writer)
          .write(&mut writer.pbr_sg_mat_writer)
          .into();
      }
      let material = material.unwrap_or(default_mat);

      StandardModelDataView::new(
        SceneMaterialDataView::PbrSGMaterial(material),
        attribute_mesh,
      )
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
  // the obj Ks is the phong specular intensity rather than the f0, using it as the f0 makes the
  // surface look like metal and darkens the diffuse by 1 - max(f0), so the obj material is
  // treated as dielectric
  mat.specular = Vec3::splat(0.04);
  if let Some(shininess) = m.shininess {
    mat.glossiness = phong_shininess_to_glossiness(shininess);
  }
  if let Some(diffuse_texture) = &m.diffuse_texture {
    let diffuse_texture = load_tex(diffuse_texture);
    mat.albedo_texture = writer
      .texture_sample_pair_writer()
      .write_direct_tex_with_default_sampler(diffuse_texture)
      .into();
  }
  if let Some(specular_texture) = &m.specular_texture {
    // the specular map modulates the dielectric f0, and the glossiness channel will default to
    // one and that's ok
    let specular_texture = load_tex(specular_texture);
    mat.specular_glossiness_texture = writer
      .texture_sample_pair_writer()
      .write_direct_tex_with_default_sampler(specular_texture)
      .into();
  }
  if let Some(normal_texture) = &m.normal_texture {
    let mut normal_texture = load_tex(normal_texture);
    // the normal map stores the direction rather than the color
    normal_texture.format = normal_texture.format.remove_srgb_suffix();
    mat.normal_texture = Some(NormalMappingDataView {
      scale: 1.0,
      content: writer
        .texture_sample_pair_writer()
        .write_direct_tex_with_default_sampler(normal_texture),
    });
  }
  mat
}

/// the phong exponent maps to the alpha roughness by sqrt(2 / (n + 2)) (Walter et al. 2007), the
/// glossiness is one minus the perceptual roughness, which is the sqrt of the alpha roughness
fn phong_shininess_to_glossiness(shininess: f32) -> f32 {
  let alpha = (2. / (shininess.max(0.) + 2.)).sqrt();
  1. - alpha.sqrt()
}

/// the area weighted vertex normal, the cross product length is twice the face area
fn compute_vertex_normals(positions: &[f32], indices: &[u32]) -> Vec<Vec3<f32>> {
  let position = |i: u32| {
    let i = i as usize * 3;
    Vec3::new(positions[i], positions[i + 1], positions[i + 2])
  };
  let mut normals = vec![Vec3::<f32>::zero(); positions.len() / 3];
  for face in indices.as_chunks::<3>().0 {
    let a = position(face[0]);
    let face_normal = (position(face[1]) - a).cross(position(face[2]) - a);
    for i in face {
      normals[*i as usize] += face_normal;
    }
  }
  normals
    .into_iter()
    .map(|n| {
      if n.length2() > 0. {
        n.normalize()
      } else {
        Vec3::new(0., 0., 1.)
      }
    })
    .collect()
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
    attributes.push((
      AttributeSemantic::Normals,
      AttributeAccessor::create_owned(
        compute_vertex_normals(&mesh.positions, &mesh.indices),
        3 * 4,
      ),
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
    mode: MeshPrimitiveTopology::TriangleList,
  }
}
