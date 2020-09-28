use rendiation_math::*;
use web_sys::*;

pub trait WebGLUploadableUniformValue {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation);
}

impl WebGLUploadableUniformValue for Mat2<f32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1fv_with_f32_array(Some(location), AsRef::<[f32; 4]>::as_ref(self));
  }
}
impl WebGLUploadableUniformValue for Mat3<f32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1fv_with_f32_array(Some(location), AsRef::<[f32; 9]>::as_ref(self));
  }
}
impl WebGLUploadableUniformValue for Mat4<f32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1fv_with_f32_array(Some(location), AsRef::<[f32; 16]>::as_ref(self));
  }
}
impl WebGLUploadableUniformValue for f32 {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1fv_with_f32_array(Some(location), &[*self; 1]);
  }
}
impl WebGLUploadableUniformValue for Vec2<f32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1fv_with_f32_array(Some(location), AsRef::<[f32; 2]>::as_ref(self));
  }
}
impl WebGLUploadableUniformValue for Vec3<f32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1fv_with_f32_array(Some(location), AsRef::<[f32; 3]>::as_ref(self));
  }
}
impl WebGLUploadableUniformValue for Vec4<f32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1fv_with_f32_array(Some(location), AsRef::<[f32; 4]>::as_ref(self));
  }
}
impl WebGLUploadableUniformValue for i32 {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1iv_with_i32_array(Some(location), &[*self; 1]);
  }
}
impl WebGLUploadableUniformValue for Vec2<i32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1iv_with_i32_array(Some(location), AsRef::<[i32; 2]>::as_ref(self));
  }
}
impl WebGLUploadableUniformValue for Vec3<i32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1iv_with_i32_array(Some(location), AsRef::<[i32; 3]>::as_ref(self));
  }
}
impl WebGLUploadableUniformValue for Vec4<i32> {
  fn upload(&self, gl: &WebGl2RenderingContext, location: &WebGlUniformLocation) {
    gl.uniform1iv_with_i32_array(Some(location), AsRef::<[i32; 4]>::as_ref(self));
  }
}
