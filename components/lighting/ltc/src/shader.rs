use crate::*;

pub struct LTCRectLight {
  pub width: f32,
  pub height: f32,
  pub double_size: bool,
}

only_fragment!(LtcLUT1, HandlePtr<ShaderTexture2D>);
only_fragment!(LtcLUT2, HandlePtr<ShaderTexture2D>);
