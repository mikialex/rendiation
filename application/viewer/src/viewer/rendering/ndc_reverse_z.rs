use crate::*;

#[derive(Clone, Copy)]
pub struct ViewerNDC {
  pub enable_reverse_z: bool,
}

/// currently, the reverse z is implement by a custom ndc space mapper.
/// this is conceptually wrong because ndc is not changed at all.
/// however it's convenient to do so because the reverse operation must implement in projection(not post transform)
/// and ndc space mapper create a good place to inject projection modification logic.
impl<T: Scalar> NDCSpaceMapper<T> for ViewerNDC {
  fn transform_from_opengl_standard_ndc(&self) -> Mat4<T> {
    let mut m = WebGPUxNDC.transform_from_opengl_standard_ndc();

    if self.enable_reverse_z {
      m.c3 = -T::half()
    }
    m
  }
}

pub fn use_ndc_reverse_z(cx: &mut Viewer3dRenderingCx) -> ViewerNDC {
  let (_, ndc) = cx.use_plain_state_init(&ViewerNDC {
    enable_reverse_z: true,
  });

  *ndc
}
