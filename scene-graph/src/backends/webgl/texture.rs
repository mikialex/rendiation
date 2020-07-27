use crate::{WebGLProgram, WebGLRenderer};
use web_sys::*;

pub struct WebGLTexture {
  texture: WebGlTexture,
  id: usize,
}

impl WebGLRenderer {
  pub fn create_texture() -> WebGLTexture {
    todo!()
  }
}

impl WebGLProgram {
  pub fn upload_texture(&self, texture: &WebGLTexture, renderer: &WebGLRenderer) {
    todo!()
  }
}

pub struct WebGLSamplerDiscriptor {}
