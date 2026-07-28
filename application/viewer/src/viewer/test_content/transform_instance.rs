use rendiation_mesh_generator::*;

use crate::*;

pub fn load_transform_instanced_wide_line_test(
  s_writer: &mut SceneWriter,
  scene: EntityHandle<SceneEntity>,
) {
  // Build source wide line mesh (sphere)
  let mesh_buffer = build_wide_line_mesh(|builder| {
    builder.build_grid_parametric(
      &SphereMeshParameter::default().make_surface(),
      TessellationConfig { u: 6, v: 6 },
      true,
    );
  });

  // Create WideLineModelEntity as the source geometry provider
  let mut wide_line_writer = global_entity_of::<WideLineModelEntity>().entity_writer();
  let wide_line_model = wide_line_writer.new_entity(|w| {
    w.write::<WideLineWidth>(&3.)
      .write::<WideLineStylePattern>(&0xffc0)
      .write::<WideLineStyleFactor>(&6.0)
      .write::<WideLineMeshBuffer>(&mesh_buffer)
  });

  // Create source SceneModel (provides geometry reference for instancing)
  let source_node = s_writer
    .node_writer
    .new_entity(|w| w.write::<SceneNodeVisibleComponent>(&false));
  // set false to avoid render it self
  // todo, check if we could set scene model visible?
  s_writer.set_local_matrix(source_node, Mat4::identity());

  let source_scene_model = s_writer.model_writer.new_entity(|w| {
    w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
      .write::<SceneModelBelongsToScene>(&scene.some_handle())
      .write::<SceneModelRefNode>(&source_node.some_handle())
  });

  // Create TransformInstancedModelEntity with 3 instances along X axis
  let instance_matrices: Vec<Mat4<f32>> = vec![
    Mat4::translate((0.0_f32, 0.0, 0.0)),
    Mat4::translate((3.0_f32, 0.0, 0.0)),
    Mat4::translate((6.0_f32, 0.0, 0.0)),
  ];
  let instance_buffer = ExternalRefPtr::new(instance_matrices);

  let mut transform_instanced_writer =
    global_entity_of::<TransformInstancedModelEntity>().entity_writer();
  let transform_instanced_model = transform_instanced_writer.new_entity(|w| {
    w.write::<TransformInstancedModelInstanceBuffer>(&instance_buffer)
      .write::<TransformInstancedModelRefSceneModel>(&source_scene_model.some_handle())
  });

  // Create instanced SceneModel (entry point for instanced rendering)
  let instanced_node = s_writer.create_root_child();
  s_writer.set_local_matrix(instanced_node, Mat4::translate((0., 2., 0.)));

  s_writer.model_writer.new_entity(|w| {
    w.write::<SceneModelTransformInstancedModelPayload>(&transform_instanced_model.some_handle())
      .write::<SceneModelBelongsToScene>(&scene.some_handle())
      .write::<SceneModelRefNode>(&instanced_node.some_handle())
  });
}
