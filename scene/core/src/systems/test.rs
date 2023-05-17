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

  let model1 = SceneModelImpl {
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

  scene.insert_model(model1);

  d_sys.maintain();
  do_updates(&mut output, |delta| {
    println!("final output delta: {delta:?}");
    let a = 1;
    let d = delta;
  })

  // scene.listen_by(all_delta).map(|d|{
  //   // d
  // })

  // let (target_scene, target_d_sys) = SceneInner::new();
  // target_scene
}
