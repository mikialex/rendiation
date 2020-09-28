use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{TextureSlotStates, WebGLProgram, WebGLRenderer, WebGLTextureBindType};
use rendiation_ral::UniformTypeId;
use web_sys::*;

static TEXTURE_GUID: AtomicUsize = AtomicUsize::new(0);

pub struct WebGLTexture {
  pub(crate) texture: WebGlTexture,
  pub(crate) ty: WebGLTextureBindType,
  pub(crate) id: usize,
}

pub trait WebGLTextureSource {
  fn get_bind_type(&self) -> WebGLTextureBindType;
  fn upload_gpu(&self, renderer: &WebGLRenderer, gpu: &WebGlTexture);
}

impl WebGLRenderer {
  pub fn create_texture(&self, source: Box<dyn WebGLTextureSource>) -> WebGLTexture {
    let texture = self.gl.create_texture().unwrap();
    source.upload_gpu(self, &texture);
    WebGLTexture {
      texture,
      ty: source.get_bind_type(),
      id: TEXTURE_GUID.fetch_add(1, Ordering::Relaxed),
    }
  }

  pub fn delete_texture(&self, tex: WebGLTexture) {
    self.gl.delete_texture(Some(&tex.texture));
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
    // self.upload_uniform_value(&UniformValue::Int(slot), texture_id, gl)
  }
}

pub struct WebGLSamplerDescriptor {}
