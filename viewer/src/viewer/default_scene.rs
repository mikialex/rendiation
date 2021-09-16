use rendiation_algebra::*;
use rendiation_renderable_mesh::{
  group::MeshDrawGroup,
  tessellation::{CubeMeshParameter, IndexedMeshTessellator, SphereMeshParameter},
};
use rendiation_texture::{rgb_to_rgba, TextureSampler, WrapAsTexture2DSource};

use crate::*;

pub fn load_default_scene(scene: &mut Scene) {
  use image::io::Reader as ImageReader;
  let path = if cfg!(windows) {
    "C:/Users/mk/Desktop/rrf-resource/planets/earth_atmos_2048.jpg"
  } else {
    todo!()
  };
  let img = ImageReader::open(path).unwrap().decode().unwrap();
  let texture = match img {
    image::DynamicImage::ImageRgba8(img) => scene.add_texture2d(img.into_source()),
    image::DynamicImage::ImageRgb8(img) => scene.add_texture2d(rgb_to_rgba(img).into_source()),
    _ => panic!("unsupport texture type"),
  };

  {
    let mesh = SphereMeshParameter::default().tessellate();
    let mesh = scene.add_mesh(mesh);
    let material = BasicMaterial {
      color: Vec3::splat(1.),
      sampler: TextureSampler::default(),
      texture,
      states: Default::default(),
    };
    let material = scene.add_material(material);

    let model = MeshModel {
      material,
      mesh,
      group: MeshDrawGroup::Full,
      node: scene.get_root_handle(),
    };

    scene.add_model(model);
  }

  {
    let mesh = CubeMeshParameter::default().tessellate();
    let mesh = scene.add_mesh(mesh);
    let mut material = BasicMaterial {
      color: Vec3::splat(1.),
      sampler: TextureSampler::default(),
      texture,
      states: Default::default(),
    };
    material.states.depth_compare = wgpu::CompareFunction::Always;
    let material = scene.add_material(material);

    let model = MeshModel {
      material,
      mesh,
      group: MeshDrawGroup::Full,
      node: scene.get_root_handle(),
    };

    scene.add_model(model);
  }

  let camera = PerspectiveProjection::default();
  let camera_node = scene.create_node(|node, _| {
    node.local_matrix = Mat4::lookat(Vec3::splat(10.), Vec3::splat(0.), Vec3::new(0., 1., 0.));
  });
  let camera = Camera::new(camera, camera_node);
  scene.active_camera = camera.into();
}
