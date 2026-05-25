use crate::*;

pub fn use_area_light_uniform(cx: &mut QueryGPUHookCx) -> Option<SceneAreaLightingProvider> {
  let uniform = use_area_per_scene_uniform_array_buffers(cx);

  let (cx, lut) = cx.use_gpu_init(|gpu, _| {
    let ltc_1 = include_bytes!("./ltc_1.bin");
    let ltc_1 = create_gpu_texture2d(
      gpu,
      &GPUBufferImage {
        data: ltc_1.as_slice().to_vec(),
        format: TextureFormat::Rgba16Float,
        size: Size::from_u32_pair_min_one((64, 64)),
      },
    );
    let ltc_2 = include_bytes!("./ltc_2.bin");
    let ltc_2 = create_gpu_texture2d(
      gpu,
      &GPUBufferImage {
        data: ltc_2.as_slice().to_vec(),
        format: TextureFormat::Rgba16Float,
        size: Size::from_u32_pair_min_one((64, 64)),
      },
    );
    (ltc_1, ltc_2)
  });

  cx.when_render(|| -> _ {
    SceneAreaLightingProvider {
      ltc_1: lut.0.clone(),
      ltc_2: lut.1.clone(),
      uniform: uniform.unwrap().1.make_read_holder(),
    }
  })
}
