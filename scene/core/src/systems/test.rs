#[test]
fn test_instance() {
  use reactive::do_updates;
  use rendiation_renderable_mesh::MeshDrawGroup;

  use crate::*;

  let (scene, mut d_sys) = SceneInner::new();

  let (_, mut output) = AutoInstanceSystem::new(scene.unbound_listen_by(all_delta), &d_sys);

  let mesh = AttributesMesh {
    attributes: Default::default(),
    indices: Default::default(),
    mode: rendiation_renderable_mesh::PrimitiveTopology::LineList,
  }
  .into_ref();
  let mesh = SceneMeshType::AttributesMesh(mesh);

  let material = FlatMaterial {
    color: Vec4::one(),
    ext: Default::default(),
  }
  .into_ref();
  let material = SceneMaterialType::Flat(material);
  let material_c = material.clone();

  let model1 = SceneModelImpl {
    model: ModelType::Standard(
      StandardModel {
        material: material.clone(),
        mesh: mesh.clone(),
        group: MeshDrawGroup::Full,
        skeleton: None,
      }
      .into_ref(),
    ),
    node: scene.create_root_child(),
  }
  .into_ref();

  let model2 = SceneModelImpl {
    model: ModelType::Standard(
      StandardModel {
        material,
        mesh,
        group: MeshDrawGroup::Full,
        skeleton: None,
      }
      .into_ref(),
    ),
    node: scene.create_root_child(),
  }
  .into_ref();
  let model2_c = model2.clone();

  scene.insert_model(model1);
  scene.insert_model(model2);

  d_sys.maintain();
  do_updates(&mut output, |delta| {
    println!("first output delta: {delta:?}");
  });
  println!("=============");

  let material2 = FlatMaterial {
    color: Vec4::one(),
    ext: Default::default(),
  }
  .into_ref();
  let material2 = SceneMaterialType::Flat(material2);
  {
    let model2_c = model2_c.read();
    if let ModelType::Standard(model) = &model2_c.model {
      model.mutate(|mut model| model.modify(StandardModelDelta::material(material2)))
    }
  }

  d_sys.maintain();
  do_updates(&mut output, |delta| {
    println!("second output delta: {delta:?}");
  });
  println!("=============");

  {
    let model2_c = model2_c.read();
    if let ModelType::Standard(model) = &model2_c.model {
      model.mutate(|mut model| model.modify(StandardModelDelta::material(material_c)))
    }
  }

  d_sys.maintain();
  do_updates(&mut output, |delta| {
    println!("third output delta: {delta:?}");
  });
  println!("=============");
}
