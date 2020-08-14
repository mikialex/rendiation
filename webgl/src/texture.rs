use crate::{TextureSlotStates, UniformValue, WebGLProgram, WebGLRenderer, WebGLTextureBindType};
use rendiation_ral::UniformTypeId;
use web_sys::*;

pub struct WebGLTexture {
  pub(crate) texture: WebGlTexture,
  pub(crate) ty: WebGLTextureBindType,
  pub(crate) id: usize,
}

impl WebGLRenderer {
  pub fn create_texture() -> WebGLTexture {
    todo!()
  }
}

impl WebGLProgram {
  pub fn use_texture(
    &self,
    texture: &WebGLTexture,
    texture_id: UniformTypeId,
    texture_slot_states: &mut TextureSlotStates,
    gl: &WebGl2RenderingContext,
  ) {
    let slot = texture_slot_states.get_free_slot().unwrap() as i32;
    texture_slot_states.bind_texture(&texture, gl);
    self.upload_uniform_value(&UniformValue::Int(slot), texture_id, gl)
  }
}

pub struct WebGLSamplerDiscriptor {}
