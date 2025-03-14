use crate::*;

fn create_gpu_texture_by_fn(
  size: Size,
  pixel: impl Fn(usize, usize) -> Vec4<f32>,
) -> GPUBufferImage {
  let mut data: Vec<u8> = vec![0; size.area() * 4];
  let s = size.into_usize();
  for y in 0..s.1 {
    for x in 0..s.0 {
      let pixel = pixel(x, y);
      data[(y * s.0 + x) * 4] = (255.).min(pixel.x * 255.) as u8;
      data[(y * s.0 + x) * 4 + 1] = (255.).min(pixel.y * 255.) as u8;
      data[(y * s.0 + x) * 4 + 2] = (255.).min(pixel.z * 255.) as u8;
      data[(y * s.0 + x) * 4 + 3] = (255.).min(pixel.w * 255.) as u8;
    }
  }

  GPUBufferImage {
    data,
    format: TextureFormat::Rgba8UnormSrgb,
    size,
  }
}

pub fn textured_example_tex(scene: &mut SceneWriter) -> Texture2DWithSamplingDataView {
  let width = 256;

  // https://lodev.org/cgtutor/xortexture.html
  let tex = create_gpu_texture_by_fn(Size::from_u32_pair_min_one((width, width)), |x, y| {
    let c = (x as u8) ^ (y as u8);
    let r = 255 - c;
    let g = c;
    let b = c % 128;

    fn channel(c: u8) -> f32 {
      c as f32 / 255.
    }

    Vec4::new(channel(r), channel(g), channel(b), 1.)
  });

  scene
    .texture_sample_pair_writer()
    .write_tex_with_default_sampler(tex)
}

pub fn load_example_cube_tex(writer: &mut SceneWriter) -> EntityHandle<SceneTextureCubeEntity> {
  // use rendiation_texture_loader::load_tex;
  // let path = if cfg!(windows) {
  //   [
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/posx.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/negx.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/posy.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/negy.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/posz.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/negz.jpg",
  //   ]
  // } else {
  //   [
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/px.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/nx.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/py.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/ny.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/pz.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/nz.png",
  //   ]
  // };
  // let x_pos = load_tex(path[0]);
  // let y_pos = load_tex(path[2]);
  // let z_pos = load_tex(path[4]);
  // let x_neg = load_tex(path[1]);
  // let y_neg = load_tex(path[3]);
  // let z_neg = load_tex(path[5]);

  let width = 256;
  // simple grid texture with gradient background
  let tex = create_gpu_texture_by_fn(Size::from_u32_pair_min_one((width, width)), |x, y| {
    let u = x as f32 / width as f32;
    let v = y as f32 / width as f32;

    if x % 25 == 0 || y % 25 == 0 {
      return Vec4::new(0., 0., 0., 1.);
    }

    Vec4::new(0., u, v, 1.)
  });

  writer.cube_texture_writer().write_cube_tex(
    tex.clone(),
    tex.clone(),
    tex.clone(),
    tex.clone(),
    tex.clone(),
    tex.clone(),
  )
}
