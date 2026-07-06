use crate::*;

/// A pair of SceneModelEntity and SceneNodeEntity that are created together
/// and must be destroyed together.
pub struct SceneModelWithUniqueNode {
  pub model: EntityHandle<SceneModelEntity>,
  pub node: EntityHandle<SceneNodeEntity>,
}

impl SceneModelWithUniqueNode {
  pub fn destroy(self, writer: &mut SceneWriter) {
    writer.model_writer.delete_entity(self.model);
    writer.node_writer.delete_entity(self.node);
  }
}

/// Directional light with its positioning node.
pub struct DirectionalLightWithNode {
  pub entity: EntityHandle<DirectionalLightEntity>,
  pub node: EntityHandle<SceneNodeEntity>,
}

impl DirectionalLightWithNode {
  pub fn destroy(self, writer: &mut SceneWriter) {
    writer.directional_light_writer.delete_entity(self.entity);
    writer.node_writer.delete_entity(self.node);
  }
}

/// Point light with its positioning node.
pub struct PointLightWithNode {
  pub entity: EntityHandle<PointLightEntity>,
  pub node: EntityHandle<SceneNodeEntity>,
}

impl PointLightWithNode {
  pub fn destroy(self, writer: &mut SceneWriter) {
    writer.point_light_writer.delete_entity(self.entity);
    writer.node_writer.delete_entity(self.node);
  }
}

/// Spot light with its positioning node.
pub struct SpotLightWithNode {
  pub entity: EntityHandle<SpotLightEntity>,
  pub node: EntityHandle<SceneNodeEntity>,
}

impl SpotLightWithNode {
  pub fn destroy(self, writer: &mut SceneWriter) {
    writer.spot_light_writer.delete_entity(self.entity);
    writer.node_writer.delete_entity(self.node);
  }
}

/// A set of two directional lights, sharing the same parameters as
/// `load_default_scene_lighting_test`.
pub struct CommonTestLights {
  pub lights: Vec<DirectionalLightWithNode>,
}

impl CommonTestLights {
  pub fn new(writer: &mut SceneWriter) -> Self {
    let light1 = {
      let node = writer.create_root_child();
      writer.set_local_matrix(node, Mat4::lookat(Vec3::splat(100.), Vec3::splat(0.), UP));
      let entity = DirectionalLightDataView {
        illuminance: Vec3::splat(5.),
        node,
        scene: writer.expect_target_scene(),
      }
      .write(&mut writer.directional_light_writer);
      DirectionalLightWithNode { entity, node }
    };

    let light2 = {
      let node = writer.create_root_child();
      writer.set_local_matrix(
        node,
        Mat4::lookat(Vec3::new(30., 100., -30.), Vec3::splat(0.), UP),
      );
      let entity = DirectionalLightDataView {
        illuminance: Vec3::new(5., 3., 2.) * 5.,
        node,
        scene: writer.expect_target_scene(),
      }
      .write(&mut writer.directional_light_writer);
      DirectionalLightWithNode { entity, node }
    };

    Self {
      lights: vec![light1, light2],
    }
  }

  pub fn destroy(self, writer: &mut SceneWriter) {
    for light in self.lights {
      light.destroy(writer);
    }
  }
}
