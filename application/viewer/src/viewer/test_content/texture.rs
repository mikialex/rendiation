use crate::*;

pub fn textured_example_tex(
  scene: &mut SceneWriter,
  texture_data_source: &mut ViewerTextureDataSource,
) -> Texture2DWithSamplingDataView {
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

  let tex = texture_data_source.create_maybe_uri_for_direct_data_dyn(Arc::new(tex));

  scene
    .texture_sample_pair_writer()
    .write_tex_with_default_sampler(tex)
}
