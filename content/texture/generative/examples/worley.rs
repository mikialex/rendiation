use rendiation_algebra::*;
use rendiation_generative_texture::{worley::*, TextureGenerator};
use rendiation_texture::*;

fn main() {
  let width = 800;
  let height = 800;

  let mut image_data = image::ImageBuffer::new(width, height);

  let worley_noise = WorleyNoise::new(100);

  image_data.fill_by(|p| {
    let value = worley_noise.gen(p);
    let value = (value * 255.).ceil() as u8;
    image::Rgb([value, value, value])
  });

  // Iterate over the coordinates and pixels of the image
  for (x, y, pixel) in image_data.enumerate_pixels_mut() {
    let value = worley_noise.get(Vec3::new(x as f32, y as f32, 0.0));
    let value = (value * 255.).ceil() as u8;
    *pixel = image::Rgb([value, value, value]);
  }

  image_data.save("worley.png").unwrap();
}
