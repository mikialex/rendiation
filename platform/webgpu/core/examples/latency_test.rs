//! Measure the minimal cpu-gpu round trip latency: submit a trivial compute work, and wait until
//! the cpu receives its result.
//!
//! The gpu can not encode commands by itself, so when the next gpu work depends on the result
//! of previous gpu work (and can not be expressed by indirect draw/dispatch), the cpu has to read
//! back the result, then encode and submit the following work. The latency measured here is the
//! lower bound of such a round trip, which decides whether this kind of cpu-gpu interleaved
//! algorithm is affordable, for example how many round trips can fit into one frame.
//!
//! The spin polling is enabled to exclude the delay introduced by the default interval polling of
//! the instance poller, so the result reflects the latency of the driver and the gpu itself.

use rendiation_shader_api::*;
use rendiation_webgpu::*;

#[pollster::main]
pub async fn main() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  // see the module doc for why spin polling is required
  gpu.instance.enable_spin_polling();

  let workgroup_size: u32 = 1;
  let init = ZeroedArrayByArrayLength(1);
  let output = create_gpu_read_write_storage::<[u32]>(init, &gpu, "output");

  let pipeline = {
    let mut cx = compute_shader_builder(&gpu).with_config_work_group_size(workgroup_size);

    let output = cx.bind_by(&output);
    let global_id = cx.global_invocation_id().x();

    output.index(global_id).store(global_id);
    cx.create_compute_pipeline(&gpu, "test").unwrap()
  };

  for _ in 0..50 {
    let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
      BindingBuilder::default()
        .with_bind(&output)
        .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
      pass.dispatch_workgroups(1, 1, 1);
    });
    let result = encoder.read_buffer(&gpu.device, &output);

    let submit_time = std::time::Instant::now();
    // move submit inside to take zero(almost) content submit cost into account.
    gpu.submit_encoder(encoder);

    let _ = result.await.unwrap();
    let get_result_time = std::time::Instant::now();

    println!("latency: {:?}", get_result_time - submit_time);
  }
}
