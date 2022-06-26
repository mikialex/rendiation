use rendiation_algebra::*;
use rendiation_color::{LinearRGBColor, SRGBColor};
use rendiation_texture::*;

pub struct Frame {
  pub inner: Texture2DBuffer<LinearRGBColor<f32>>,
}

impl Frame {
  pub fn new(width: usize, height: usize) -> Frame {
    Frame {
      inner: Texture2DBuffer::init_with(
        Size::from_usize_pair_min_one((width, height)),
        LinearRGBColor::new(0., 0., 0.),
      ),
    }
  }

  pub fn size(&self) -> Vec2<usize> {
    (self.width(), self.height()).into()
  }

  pub fn width(&self) -> usize {
    self.inner.width()
  }
  pub fn height(&self) -> usize {
    self.inner.height()
  }

  pub fn clear(&mut self, color: LinearRGBColor<f32>) {
    self.inner.clear(color)
  }

  pub fn set_pixel(&mut self, color: LinearRGBColor<f32>, x: usize, y: usize) {
    self.inner.write(vector!(x, y), color)
  }

  pub fn pixel_count(&self) -> usize {
    self.width() * self.height()
  }

  pub fn write_result(&self, name: &str) {
    let mut current_path = std::env::current_dir().unwrap();
    println!("working dir {}", current_path.display());
    current_path.push(String::from(name) + ".png");
    let path = current_path.into_os_string().into_string().unwrap();

    println!("writing file to path: {}", path);
    self
      .inner
      .map::<OutputBuffer>(|pix| {
        // todo, should I invent two dimension iter?
        let pix: SRGBColor<f32> = pix.into();
        image::Rgba([
          (pix.r.min(1.0).max(0.0) * 255.0) as u8,
          (pix.g.min(1.0).max(0.0) * 255.0) as u8,
          (pix.b.min(1.0).max(0.0) * 255.0) as u8,
          255,
        ])
      })
      .save(path.clone())
      .unwrap();
    println!("{} pixels has write to {}", self.pixel_count(), path);
  }
}

type OutputBuffer = image::ImageBuffer<image::Rgba<u8>, Vec<u8>>;
