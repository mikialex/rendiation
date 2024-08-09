use rendiation_device_ray_tracing::*;
use rendiation_webgpu::*;

#[pollster::main]
async fn main() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  //
}
