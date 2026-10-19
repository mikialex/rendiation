use super::*;

/// `PLANE`: `o + u·x + v·y`.
pub struct Plane {
  frame: Frame,
}

impl Plane {
  pub fn new(frame: Frame) -> Self {
    Self { frame }
  }
}

impl SurfaceModel for Plane {
  fn kind_name(&self) -> &'static str {
    "Plane"
  }

  fn eval(&self, native: Vec2<f64>) -> Vec3<f64> {
    self.frame.origin + self.frame.x * native.x + self.frame.y * native.y
  }

  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    (self.eval(native), self.frame.x, self.frame.y)
  }

  fn period(&self) -> [Option<f64>; 2] {
    [None, None]
  }

  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [None, None]
  }

  fn poles(&self) -> Vec<Pole> {
    Vec::new()
  }

  fn project(&self, p: Vec3<f64>, _guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    let d = p - self.frame.origin;
    let native = Vec2::new(d.dot(self.frame.x), d.dot(self.frame.y));
    let distance = d.dot(self.frame.z).abs();
    finite_projection(native, distance)
  }

  fn build_param_space(
    &self,
    rect: NativeRect,
    _ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    let u_map = affine_map(rect.u)?;
    let v_map = affine_map(rect.v)?;
    let mut cp = Vec::with_capacity(4);
    for v in [rect.v.0, rect.v.1] {
      for u in [rect.u.0, rect.u.1] {
        cp.push(Vec4::from_euclid(self.eval(Vec2::new(u, v)), 1.));
      }
    }
    Ok(SurfaceParamSpace {
      u_map,
      v_map,
      patches: vec![CellPatch {
        cell: (0, 0),
        surface: RationalBezierSurface::new(1, 1, cp),
      }],
    })
  }
}
