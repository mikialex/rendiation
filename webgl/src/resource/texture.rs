use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{WebGLRenderer, WebGLTextureBindType};
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

pub struct WebGLSamplerDescriptor {}
