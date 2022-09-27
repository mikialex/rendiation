use image::*;
use rendiation_algebra::*;
use rendiation_mesh_generator::{
  CubeMeshParameter, IndexedMeshBuilder, SphereMeshParameter, TessellationConfig,
};
use rendiation_renderable_mesh::{vertex::Vertex, TriangleList};
use rendiation_texture::{rgb_to_rgba, TextureSampler, WrapAsTexture2DSource};
use webgpu::WebGPUTexture2dSource;

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

pub fn load_img_cube() -> <WebGPUScene as SceneContent>::TextureCube {
  let path = [
    "C:/Users/mk/Desktop/rrf-resource/Park2/posx.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/negx.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/posy.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/negy.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/posz.jpg",
    "C:/Users/mk/Desktop/rrf-resource/Park2/negz.jpg",
  ];

  fn load(path: &&str) -> Box<dyn WebGPUTexture2dSource> {
    Box::new(load_img(path).into_source())
  }

  // https://github.com/rust-lang/rust/issues/81615
  path
    .iter()
    .map(load)
    .collect::<Vec<_>>()
    .try_into()
    .unwrap()
}

pub fn load_default_scene(scene: &mut Scene<WebGPUScene>) {
  scene.background = Some(Box::new(SolidBackground {
    intensity: Vec3::new(0.1, 0.1, 0.1),
  }));

  let path = if cfg!(windows) {
    "C:/Users/mk/Desktop/rrf-resource/planets/earth_atmos_2048.jpg"
  } else {
    "/Users/mikialex/Desktop/test.png"
  };

  let texture = SceneTexture2D::<WebGPUScene>::new(Box::new(load_img(path).into_source()));
  let texture = TextureWithSamplingData {
    texture,
    sampler: TextureSampler::default(),
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
      .build_mesh_into();
    let mesh = MeshSource::new(mesh);
    let material = PhysicalMaterial::<WebGPUScene> {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.clone().into(),
    }
    .use_state();

    let child = scene.root().create_child();
    child.set_local_matrix(Mat4::translate((2., 0., 3.)));

    let model: MeshModel<_, _> = MeshModelImpl::new(material, mesh, child).into();
    let _ = scene.add_model(model);
    // let model_handle = scene.add_model(model);
    // scene.remove_model(model_handle);
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
    let mesh = builder.build_mesh();
    let mesh = MeshSource::new(mesh);
    let material = PhysicalMaterial::<WebGPUScene> {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.clone().into(),
    }
    .use_state();
    let child = scene.root().create_child();

    let model: MeshModel<_, _> = MeshModelImpl::new(material, mesh, child).into();
    let _ = scene.add_model(model);
  }

  {
    let mesh = IndexedMeshBuilder::<TriangleList, Vec<Vertex>>::default()
      .triangulate_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 16, v: 16 },
        true,
      )
      .build_mesh_into();
    let mesh = MeshSource::new(mesh);

    let mesh = TransformInstance {
      mesh,
      transforms: vec![
        Mat4::translate((10., 0., 0.)),
        Mat4::translate((10., 0., 2.)),
        Mat4::translate((10., 0., 4.)),
        Mat4::translate((10., 0., 6.)),
      ],
    };
    let material = PhysicalMaterial::<WebGPUScene> {
      albedo: Vec3::splat(1.),
      albedo_texture: texture.clone().into(),
    }
    .use_state();

    let model: MeshModel<_, _> =
      MeshModelImpl::new(material, mesh, scene.root().create_child()).into();
    let _ = scene.add_model(model);
  }

  let up = Vec3::new(0., 1., 0.);

  {
    let camera = PerspectiveProjection::default();
    let camera_node = scene.root().create_child();
    camera_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
    let camera = SceneCamera::create_camera(camera, camera_node);
    scene.active_camera = camera.into();
  }

  {
    let camera = PerspectiveProjection::default();
    let camera_node = scene.root().create_child();
    camera_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
    let camera = SceneCamera::create_camera(camera, camera_node);
    let _ = scene.add_camera(camera);
  }

  let directional_light_node = scene.root().create_child();
  directional_light_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
  let directional_light = DirectionalLight {
    intensity: Vec3::splat(5.),
  };
  let directional_light = SceneLightInner {
    light: Box::new(directional_light) as Box<dyn WebGPUSceneLight>,
    node: directional_light_node,
  };
  let directional_light = SceneItemRef::new(directional_light);
  scene.lights.insert(directional_light);

  let directional_light_node = scene.root().create_child();
  directional_light_node.set_local_matrix(Mat4::lookat(
    Vec3::new(3., 3., -3.),
    Vec3::splat(0.),
    up,
  ));
  let directional_light = DirectionalLight {
    intensity: Vec3::new(5., 3., 2.),
  };
  let directional_light = SceneLightInner {
    light: Box::new(directional_light) as Box<dyn WebGPUSceneLight>,
    node: scene.root().create_child(),
  };
  let directional_light = SceneItemRef::new(directional_light);
  scene.lights.insert(directional_light);

  rendiation_scene_gltf_loader::load_gltf_test(
    // "C:/Users/mk/Desktop/develop/glTF-Sample-Models/2.0/Suzanne/glTF/Suzanne.gltf",
    // "C:/Users/mk/Desktop/develop/glTF-Sample-Models/2.0/Sponza/glTF/Sponza.gltf",
    "/Users/mikialex/dev/glTF-Sample-Models/2.0/Box/glTF/Box.gltf",
    scene,
  )
  .unwrap();
}
