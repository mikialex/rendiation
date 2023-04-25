use std::sync::Arc;

use image::*;
use rendiation_algebra::*;
use rendiation_mesh_generator::{
  CubeMeshParameter, IndexedMeshBuilder, SphereMeshParameter, TessellationConfig,
};
use rendiation_renderable_mesh::{vertex::Vertex, TriangleList};
use rendiation_texture::{rgb_to_rgba, TextureSampler, WrapAsTexture2DSource};
use webgpu::WebGPU2DTextureSource;

use crate::*;

pub fn load_img(path: &str) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
  use image::io::Reader as ImageReader;
  let img = ImageReader::open(path).unwrap().decode().unwrap();
  match img {
    image::DynamicImage::ImageRgba8(img) => img,
    image::DynamicImage::ImageRgb8(img) => rgb_to_rgba(img),
    _ => panic!("unsupported texture type"),
  }
}
fn load_tex(path: &str) -> SceneTexture2DType {
  let boxed: Box<dyn WebGPU2DTextureSource> = Box::new(load_img(path).into_source());
  SceneTexture2DType::Foreign(Arc::new(boxed))
}

pub fn load_img_cube() -> SceneTextureCube {
  let path = [
    "C:/Users/mk/Desktop/rrf-resource/Park2/posx.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/negx.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/posy.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/negy.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/posz.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/negz.jpg",
  ];

  SceneTextureCubeImpl {
    faces: path.map(load_tex),
  }
  .into()
}

pub fn load_default_scene(scene: &Scene) {
  scene.set_background(Some(SceneBackGround::Solid(SolidBackground {
    intensity: Vec3::new(0.1, 0.1, 0.1),
  })));

  let path = if cfg!(windows) {
    "C:/Users/mk/Desktop/rrf-resource/planets/earth_atmos_2048.jpg"
  } else {
    "/Users/mikialex/Desktop/test.png"
  };

  let texture = TextureWithSamplingData {
    texture: load_tex(path).into_ref(),
    sampler: TextureSampler::tri_linear_repeat(),
  };

  // let texture_cube = scene.add_texture_cube(load_img_cube());

  // let background_mat = EnvMapBackGroundMaterial {
  //   sampler: TextureSampler::default(),
  //   texture: texture_cube,
  // };
  // let background_mat = scene.add_material(background_mat);
  // let bg = DrawableBackground::new(background_mat);

  // scene.background = Box::new(bg);

  {
    let mesh = IndexedMeshBuilder::<TriangleList, Vec<Vertex>>::default()
      .triangulate_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 16, v: 16 },
        true,
      )
      .build_mesh_into()
      .into_ref();
    let mesh: Box<dyn WebGPUSceneMesh> = Box::new(mesh);
    let mesh = SceneMeshType::Foreign(Arc::new(mesh));

    let material = PhysicalSpecularGlossinessMaterial {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.clone().into(),
      ..Default::default()
    };
    let material = SceneMaterialType::PhysicalSpecularGlossiness(material.into());

    let child = scene.read().root().create_child();
    child.set_local_matrix(Mat4::translate((2., 0., 3.)));

    let model = StandardModel::new(material, mesh);
    let model = ModelType::Standard(model.into());
    let model = SceneModelImpl { model, node: child };
    let _ = scene.insert_model(model.into());
  }

  {
    let cube = CubeMeshParameter {
      width: 1.,
      height: 2.,
      depth: 3.,
    };
    let mut builder = IndexedMeshBuilder::<TriangleList, Vec<Vertex>>::default();
    for face in cube.make_faces() {
      builder = builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
    }
    let mesh = builder.build_mesh().into_ref();
    let mesh: Box<dyn WebGPUSceneMesh> = Box::new(mesh);
    let mesh = SceneMeshType::Foreign(Arc::new(mesh));

    let material = PhysicalSpecularGlossinessMaterial {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.clone().into(),
      ..Default::default()
    };
    let material = SceneMaterialType::PhysicalSpecularGlossiness(material.into());
    let child = scene.read().root().create_child();

    let model = StandardModel::new(material, mesh);
    let model = ModelType::Standard(model.into());
    let model = SceneModelImpl { model, node: child };
    let _ = scene.insert_model(model.into());
  }

  {
    let mesh = IndexedMeshBuilder::<TriangleList, Vec<Vertex>>::default()
      .triangulate_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 16, v: 16 },
        true,
      )
      .build_mesh_into()
      .into_ref();
    let mesh: Box<dyn WebGPUSceneMesh> = Box::new(mesh);
    let mesh = SceneMeshType::Foreign(Arc::new(mesh));

    let mesh = TransformInstancedSceneMesh {
      mesh,
      transforms: vec![
        Mat4::translate((10., 0., 0.)),
        Mat4::translate((10., 0., 2.)),
        Mat4::translate((10., 0., 4.)),
        Mat4::translate((10., 0., 6.)),
      ],
    }
    .into_ref();
    let mesh = SceneMeshType::TransformInstanced(mesh);

    let material = PhysicalSpecularGlossinessMaterial {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.into(),
      ..Default::default()
    };
    let material = SceneMaterialType::PhysicalSpecularGlossiness(material.into());
    let child = scene.read().root().create_child();

    let model = StandardModel::new(material, mesh);
    let model = ModelType::Standard(model.into());
    let model = SceneModelImpl { model, node: child };
    let _ = scene.insert_model(model.into());
  }

  let up = Vec3::new(0., 1., 0.);

  {
    let camera = PerspectiveProjection::default();
    let camera_node = scene.read().root().create_child();
    camera_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
    let camera = SceneCamera::create_camera(camera, camera_node);
    scene.set_active_camera(camera.into());
  }

  {
    let camera = PerspectiveProjection::default();
    let camera_node = scene.read().root().create_child();
    camera_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
    let camera = SceneCamera::create_camera(camera, camera_node);
    let _ = scene.insert_camera(camera);
  }

  let directional_light_node = scene.read().root().create_child();
  directional_light_node.set_local_matrix(Mat4::lookat(Vec3::splat(300.), Vec3::splat(0.), up));
  let directional_light = DirectionalLight {
    illuminance: 5.,
    color_factor: Vec3::one(),
    ext: Default::default(),
  };
  let directional_light = SceneLightKind::DirectionalLight(directional_light.into());
  let directional_light = SceneLightInner {
    light: directional_light,
    node: directional_light_node,
  };
  scene.insert_light(directional_light.into());

  let directional_light_node = scene.read().root().create_child();
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
  let directional_light = SceneLightKind::DirectionalLight(directional_light.into());
  let directional_light = SceneLightInner {
    light: directional_light,
    node: directional_light_node,
  };
  scene.insert_light(directional_light.into());

  let point_light_node = scene.read().root().create_child();
  point_light_node.set_local_matrix(Mat4::translate((2., 2., 2.)));
  let point_light = PointLight {
    color_factor: Vec3::new(5., 3., 2.) / Vec3::splat(5.),
    luminance_intensity: 5.,
    cutoff_distance: 40.,
    ext: Default::default(),
  };
  let point_light = SceneLightKind::PointLight(point_light.into());
  let point_light = SceneLightInner {
    light: point_light,
    node: point_light_node,
  };
  scene.insert_light(point_light.into());

  let spot_light_node = scene.read().root().create_child();
  spot_light_node.set_local_matrix(Mat4::lookat(Vec3::new(-5., 5., 5.), Vec3::splat(0.), up));
  let spot_light = SpotLight {
    luminance_intensity: 180.,
    color_factor: Vec3::new(1., 0., 0.),
    cutoff_distance: 40.,
    half_cone_angle: Deg::by(5. / 2.).to_rad(),
    half_penumbra_angle: Deg::by(5. / 2.).to_rad(),
    ext: Default::default(),
  };
  let spot_light = SceneLightKind::SpotLight(spot_light.into());
  let spot_light = SceneLightInner {
    light: spot_light,
    node: spot_light_node,
  };
  scene.insert_light(spot_light.into());
}
