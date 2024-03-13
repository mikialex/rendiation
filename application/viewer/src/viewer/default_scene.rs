use rendiation_algebra::*;
use rendiation_mesh_core::{vertex::Vertex, TriangleList};
use rendiation_mesh_generator::{
  CubeMeshParameter, IndexedMeshBuilder, IntoTransformed3D, SphereMeshParameter, TessellationConfig,
};
use rendiation_texture::{
  create_padding_buffer, GPUBufferImage, Texture2D, TextureFormat, TextureSampler,
};

use crate::*;

pub fn load_tex(path: &str) -> SceneTexture2DType {
  use image::io::Reader as ImageReader;
  let img = ImageReader::open(path).unwrap().decode().unwrap();
  let tex = match img {
    image::DynamicImage::ImageRgba8(img) => {
      let size = img.size();
      let format = TextureFormat::Rgba8UnormSrgb;
      let data = img.into_raw();
      GPUBufferImage { data, format, size }
    }
    image::DynamicImage::ImageRgb8(img) => {
      let size = img.size();
      let format = TextureFormat::Rgba8UnormSrgb;
      let data = create_padding_buffer(img.as_raw(), 3, &[255]);
      GPUBufferImage { data, format, size }
    }
    _ => panic!("unsupported texture type"),
  };
  SceneTexture2DType::GPUBufferImage(tex)
}

pub fn load_img_cube() -> SceneTextureCube {
  let path = if cfg!(windows) {
    [
      "C:/Users/mk/Desktop/rrf-resource/Park2/posx.jpg",
      "C:/Users/mk/Desktop/rrf-resource/Park2/negx.jpg",
      "C:/Users/mk/Desktop/rrf-resource/Park2/posy.jpg",
      "C:/Users/mk/Desktop/rrf-resource/Park2/negy.jpg",
      "C:/Users/mk/Desktop/rrf-resource/Park2/posz.jpg",
      "C:/Users/mk/Desktop/rrf-resource/Park2/negz.jpg",
    ]
  } else {
    [
      "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/px.png",
      "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/nx.png",
      "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/py.png",
      "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/ny.png",
      "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/pz.png",
      "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/nz.png",
    ]
  };

  SceneTextureCubeImpl {
    faces: path.map(load_tex),
  }
  .into()
}

type SceneMeshBuilder =
  IndexedMeshBuilder<GroupedMesh<IndexedMesh<TriangleList, Vec<Vertex>, DynIndexContainer>>>;

pub fn build_scene_mesh(f: impl FnOnce(&mut SceneMeshBuilder)) -> MeshEnum {
  let mut builder = SceneMeshBuilder::default();
  f(&mut builder);
  let mesh = builder.finish();
  let mut attribute: AttributeMeshData = mesh.mesh.primitive_iter().collect();
  attribute.groups = mesh.groups;
  MeshEnum::AttributesMesh(attribute.build().into_ptr())
}

pub fn load_default_scene(scene: &Scene) {
  let path = if cfg!(windows) {
    "C:/Users/mk/Desktop/rrf-resource/planets/earth_atmos_2048.jpg"
  } else {
    "/Users/mikialex/Desktop/test.png"
  };

  let texture = TextureWithSamplingData {
    texture: load_tex(path).into_ptr(),
    sampler: TextureSampler::tri_linear_repeat().into_ptr(),
  };

  scene.set_background(Some(SceneBackGround::Solid(SolidBackground {
    intensity: Vec3::new(0.1, 0.1, 0.1),
  })));
  // scene.set_background(Some(SceneBackGround::Env(EnvMapBackground {
  //   texture: load_img_cube(),
  // })));

  {
    let mesh = build_scene_mesh(|builder| {
      builder.triangulate_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 16, v: 16 },
        true,
      );
    });

    let material = PhysicalSpecularGlossinessMaterial {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.clone().into(),
      ..Default::default()
    };
    let material = MaterialEnum::PhysicalSpecularGlossiness(material.into());

    let child = scene.create_root_child();
    child.set_local_matrix(Mat4::translate((2., 0., 3.)));

    let model = StandardModel::new(material, mesh);
    let model = ModelEnum::Standard(model.into());
    let model = SceneModelImpl::new(model, child);
    let _ = scene.insert_model(model.into());
  }

  {
    let cube = CubeMeshParameter {
      width: 1.,
      height: 2.,
      depth: 3.,
    };
    let mesh = build_scene_mesh(|builder| {
      for face in cube.make_faces() {
        builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
      }
    });

    let material = PhysicalSpecularGlossinessMaterial {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.clone().into(),
      ..Default::default()
    };
    let material = MaterialEnum::PhysicalSpecularGlossiness(material.into());
    let child = scene.create_root_child();

    let model = StandardModel::new(material, mesh);
    let model = ModelEnum::Standard(model.into());
    let model = SceneModelImpl::new(model, child);
    let _ = scene.insert_model(model.into());
  }

  {
    let mesh = build_scene_mesh(|builder| {
      builder.triangulate_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 16, v: 16 },
        true,
      );
    });

    let mesh = TransformInstancedSceneMesh {
      mesh,
      transforms: vec![
        Mat4::translate((10., 0., 0.)),
        Mat4::translate((10., 0., 2.)),
        Mat4::translate((10., 0., 4.)),
        Mat4::translate((10., 0., 6.)),
      ],
    }
    .into_ptr();
    let mesh = MeshEnum::TransformInstanced(mesh);

    let material = PhysicalSpecularGlossinessMaterial {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.into(),
      ..Default::default()
    };
    let material = MaterialEnum::PhysicalSpecularGlossiness(material.into());
    let child = scene.create_root_child();

    let model = StandardModel::new(material, mesh);
    let model = ModelEnum::Standard(model.into());
    let model = SceneModelImpl::new(model, child);
    let _ = scene.insert_model(model.into());
  }

  let up = Vec3::new(0., 1., 0.);

  {
    let camera = PerspectiveProjection::default();
    let camera = CameraProjectionEnum::Perspective(camera);
    let camera_node = scene.create_root_child();
    camera_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
    let camera = SceneCameraImpl::new(camera, camera_node).into_ptr();
    let _ = scene.insert_camera(camera.clone());
    scene.set_active_camera(camera.into());
  }

  {
    let camera = PerspectiveProjection::default();
    let camera = CameraProjectionEnum::Perspective(camera);
    let camera_node = scene.create_root_child();
    camera_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
    let camera = SceneCameraImpl::new(camera, camera_node).into_ptr();
    let _ = scene.insert_camera(camera);
  }

  let directional_light_node = scene.create_root_child();
  directional_light_node.set_local_matrix(Mat4::lookat(Vec3::splat(300.), Vec3::splat(0.), up));
  let directional_light = DirectionalLight {
    illuminance: 5.,
    color_factor: Vec3::one(),
    ext: Default::default(),
  };
  let directional_light = LightEnum::DirectionalLight(directional_light.into());
  let directional_light = SceneLightImpl::new(directional_light, directional_light_node);
  scene.insert_light(directional_light.into());

  let directional_light_node = scene.create_root_child();
  directional_light_node.set_local_matrix(Mat4::lookat(
    Vec3::new(30., 300., -30.),
    Vec3::splat(0.),
    up,
  ));
  let directional_light = DirectionalLight {
    illuminance: 5.,
    color_factor: Vec3::new(5., 3., 2.) / Vec3::splat(5.),
    ext: Default::default(),
  };
  let directional_light = LightEnum::DirectionalLight(directional_light.into());
  let directional_light = SceneLightImpl::new(directional_light, directional_light_node);
  scene.insert_light(directional_light.into());

  let point_light_node = scene.create_root_child();
  point_light_node.set_local_matrix(Mat4::translate((2., 2., 2.)));
  let point_light = PointLight {
    color_factor: Vec3::new(5., 3., 2.) / Vec3::splat(5.),
    luminance_intensity: 5.,
    cutoff_distance: 40.,
    ext: Default::default(),
  };
  let point_light = LightEnum::PointLight(point_light.into());
  let point_light = SceneLightImpl::new(point_light, point_light_node);
  scene.insert_light(point_light.into());

  let spot_light_node = scene.create_root_child();
  spot_light_node.set_local_matrix(Mat4::lookat(Vec3::new(-5., 5., 5.), Vec3::splat(0.), up));
  let spot_light = SpotLight {
    luminance_intensity: 180.,
    color_factor: Vec3::new(1., 0., 0.),
    cutoff_distance: 40.,
    half_cone_angle: Deg::by(5. / 2.).to_rad(),
    half_penumbra_angle: Deg::by(5. / 2.).to_rad(),
    ext: Default::default(),
  };
  let spot_light = LightEnum::SpotLight(spot_light.into());
  let spot_light = SceneLightImpl::new(spot_light, spot_light_node);
  scene.insert_light(spot_light.into());

  // stress_test2(scene);
}

pub fn stress_test(scene: &Scene) {
  let material = PhysicalSpecularGlossinessMaterial {
    albedo: Vec3::splat(1.),
    albedo_texture: None,
    ..Default::default()
  };
  let material = MaterialEnum::PhysicalSpecularGlossiness(material.into());
  for i in 0..10 {
    let i_parent = scene.create_root_child();
    i_parent.set_local_matrix(Mat4::translate((i as f32, 0., 0.)));
    for j in 0..10 {
      let j_parent = i_parent.create_child();
      j_parent.set_local_matrix(Mat4::translate((0., 0., j as f32)));
      for k in 0..1 {
        let node = j_parent.create_child();
        node.set_local_matrix(Mat4::translate((0., k as f32, 0.)));

        let cube = CubeMeshParameter {
          width: 0.2,
          height: 0.2,
          depth: 0.2,
        };
        let mesh = build_scene_mesh(|builder| {
          for face in cube.make_faces() {
            builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
          }
        });

        let child = scene.create_root_child();
        child.set_local_matrix(Mat4::translate((2., 0., 3.)));

        let model = StandardModel::new(material.clone(), mesh);
        let model = ModelEnum::Standard(model.into());
        let model = SceneModelImpl::new(model, node);
        let _ = scene.insert_model(model.into());
      }
    }
  }
}

pub fn stress_test2(scene: &Scene) {
  let material = PhysicalSpecularGlossinessMaterial {
    albedo: Vec3::splat(1.),
    albedo_texture: None,
    ..Default::default()
  };
  let material = MaterialEnum::PhysicalSpecularGlossiness(material.into());
  for i in 0..100 {
    let i_parent = scene.create_root_child();
    for j in 0..100 {
      let j_parent = i_parent.create_child();
      for k in 0..1 {
        let node = j_parent.create_child();
        let cube = CubeMeshParameter {
          width: 0.2,
          height: 0.2,
          depth: 0.2,
        };
        let mesh = build_scene_mesh(|builder| {
          for face in cube.make_faces() {
            builder.triangulate_parametric(
              &face.transform_by(Mat4::translate((i as f32, k as f32, j as f32))),
              TessellationConfig { u: 2, v: 3 },
              true,
            );
          }
        });

        let child = scene.create_root_child();
        child.set_local_matrix(Mat4::translate((2., 0., 3.)));

        let model = StandardModel::new(material.clone(), mesh);
        let model = ModelEnum::Standard(model.into());
        let model = SceneModelImpl::new(model, node);
        let _ = scene.insert_model(model.into());
      }
    }
  }
}
