use rendiation_webgl::*;

use crate::{ShaderGraphSampler, ShaderGraphTexture};

impl WebGLUniformUploadable for ShaderGraphTexture {
  type UploadValue = i32;
  type UploadInstance = SingleUniformUploadInstance<Self::UploadValue>;
}

pub struct EmptyImpl;

impl<T> UploadInstance<T> for EmptyImpl {
  fn create(_: &str, _: &WebGl2RenderingContext, _: &WebGlProgram) -> Self {
    Self
  }
  fn upload(&mut self, _: &T, _: &WebGl2RenderingContext) {}
}

impl WebGLUniformUploadable for ShaderGraphSampler {
  type UploadValue = ();
  type UploadInstance = EmptyImpl;
}
