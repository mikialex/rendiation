use rendiation_algebra::*;
use rendiation_color::{LinearRGBColor, SRGBColor};
use rendiation_texture::{Size, Texture2D, Texture2DBuffer, Texture2dInitAble};

/// old perspective defaults
#[allow(dead_code)]
pub fn make_perspective<T: Scalar>() -> PerspectiveProjection<T> {
  PerspectiveProjection {
    near: T::eval::<{ scalar_transmute(1.0) }>(),
    far: T::eval::<{ scalar_transmute(100_1000.0) }>(),
    fov: Deg::by(T::eval::<{ scalar_transmute(90.0) }>()),
    aspect: T::eval::<{ scalar_transmute(1.0) }>(),
  }
}

pub fn make_frame(width: usize, height: usize) -> Texture2DBuffer<LinearRGBColor<f32>> {
  Texture2DBuffer::init_with(
    Size::from_usize_pair_min_one((width, height)),
    LinearRGBColor::new(0., 0., 0.),
  )
}

type OutputBuffer = image::ImageBuffer<image::Rgba<u8>, Vec<u8>>;
pub fn write_frame(frame: &Texture2DBuffer<LinearRGBColor<f32>>, name: &str) {
  let mut current_path = std::env::current_dir().unwrap();
  println!("working dir {}", current_path.display());
  current_path.push(String::from(name) + ".png");
  let path = current_path.into_os_string().into_string().unwrap();

  println!("writing file to path: {path}");
  frame
    .map::<OutputBuffer>(|pix| {
      // todo, should I invent two dimension iter?
      let pix: SRGBColor<f32> = pix.into();
      image::Rgba([
        (pix.r.clamp(0.0, 1.0) * 255.0) as u8,
        (pix.g.clamp(0.0, 1.0) * 255.0) as u8,
        (pix.b.clamp(0.0, 1.0) * 255.0) as u8,
        255,
      ])
    })
    .save(path.clone())
    .unwrap();
  println!("{} pixels has write to {}", frame.pixel_count(), path);
}

// This allows treating the utils as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {}
