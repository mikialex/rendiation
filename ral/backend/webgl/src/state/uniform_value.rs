use rendiation_algebra::*;
use rendiation_ral::ResourceManager;
use web_sys::*;

use crate::{WebGL, WebGLRenderer};

pub trait WebGLUniformUploadable: Sized {
  type UploadValue;
  type UploadInstance: UploadInstance<Self>;

  fn upload(
    value: &Self::UploadValue,
    instance: &mut Self::UploadInstance,
    renderer: &mut WebGLRenderer,
    resource: &ResourceManager<WebGL>,
  ) {
    instance.upload(value, renderer, resource)
  }
}

pub trait UploadInstance<T: WebGLUniformUploadable> {
  fn create(query_name_prefix: &str, gl: &WebGl2RenderingContext, program: &WebGlProgram) -> Self;
  fn upload(
    &mut self,
    value: &T::UploadValue,
    renderer: &mut WebGLRenderer,
    resource: &ResourceManager<WebGL>,
  );
}

pub trait SingleUniformUploadSource: PartialEq + Default + Copy {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext);
}

pub struct SingleUniformUploadInstance<T: SingleUniformUploadSource> {
  cache: T,
  location: Option<WebGlUniformLocation>,
}

impl<T: SingleUniformUploadSource> SingleUniformUploadInstance<T> {
  pub fn new(query_name_prefix: &str, gl: &WebGl2RenderingContext, program: &WebGlProgram) -> Self {
    let location = gl.get_uniform_location(program, query_name_prefix);
    Self {
      cache: T::default(),
      location,
    }
  }
  pub fn upload(&mut self, value: &T, renderer: &mut WebGLRenderer) {
    if self.cache != *value {
      self.cache = *value;
      value.upload(&self.location, &renderer.gl);
    }
  }
}

impl<T: WebGLUniformUploadable> UploadInstance<T> for SingleUniformUploadInstance<T::UploadValue>
where
  T::UploadValue: SingleUniformUploadSource,
{
  fn create(query_name_prefix: &str, gl: &WebGl2RenderingContext, program: &WebGlProgram) -> Self {
    Self::new(query_name_prefix, gl, program)
  }
  fn upload(
    &mut self,
    value: &T::UploadValue,
    renderer: &mut WebGLRenderer,
    _resource: &ResourceManager<WebGL>,
  ) {
    self.upload(value, renderer);
  }
}

macro_rules! derive_single_source {
  ($Source:ty) => {
    impl WebGLUniformUploadable for $Source {
      type UploadValue = Self;
      type UploadInstance = SingleUniformUploadInstance<Self>;
    }
  };
}

derive_single_source!(Mat2<f32>);
impl SingleUniformUploadSource for Mat2<f32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1fv_with_f32_array(location.as_ref(), AsRef::<[f32; 4]>::as_ref(self));
  }
}

derive_single_source!(Mat3<f32>);
impl SingleUniformUploadSource for Mat3<f32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1fv_with_f32_array(location.as_ref(), AsRef::<[f32; 9]>::as_ref(self));
  }
}

derive_single_source!(Mat4<f32>);
impl SingleUniformUploadSource for Mat4<f32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1fv_with_f32_array(location.as_ref(), AsRef::<[f32; 16]>::as_ref(self));
  }
}

derive_single_source!(f32);
impl SingleUniformUploadSource for f32 {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1fv_with_f32_array(location.as_ref(), &[*self; 1]);
  }
}

derive_single_source!(Vec2<f32>);
impl SingleUniformUploadSource for Vec2<f32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1fv_with_f32_array(location.as_ref(), AsRef::<[f32; 2]>::as_ref(self));
  }
}

derive_single_source!(Vec3<f32>);
impl SingleUniformUploadSource for Vec3<f32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1fv_with_f32_array(location.as_ref(), AsRef::<[f32; 3]>::as_ref(self));
  }
}

derive_single_source!(Vec4<f32>);
impl SingleUniformUploadSource for Vec4<f32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1fv_with_f32_array(location.as_ref(), AsRef::<[f32; 4]>::as_ref(self));
  }
}

derive_single_source!(i32);
impl SingleUniformUploadSource for i32 {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1iv_with_i32_array(location.as_ref(), &[*self; 1]);
  }
}

derive_single_source!(Vec2<i32>);
impl SingleUniformUploadSource for Vec2<i32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1iv_with_i32_array(location.as_ref(), AsRef::<[i32; 2]>::as_ref(self));
  }
}
derive_single_source!(Vec3<i32>);
impl SingleUniformUploadSource for Vec3<i32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1iv_with_i32_array(location.as_ref(), AsRef::<[i32; 3]>::as_ref(self));
  }
}
derive_single_source!(Vec4<i32>);
impl SingleUniformUploadSource for Vec4<i32> {
  fn upload(&self, location: &Option<WebGlUniformLocation>, gl: &WebGl2RenderingContext) {
    gl.uniform1iv_with_i32_array(location.as_ref(), AsRef::<[i32; 4]>::as_ref(self));
  }
}
