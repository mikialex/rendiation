use rendiation_shader_api::*;
use rendiation_webgpu::*;

#[pollster::main]
pub async fn main() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  gpu.instance.enable_spin_polling();

  let workgroup_size: u32 = 1;
  let init = ZeroedArrayByArrayLength(1);
  let output = create_gpu_read_write_storage::<[u32]>(init, &gpu);

  let pipeline = {
    let mut cx = compute_shader_builder().with_config_work_group_size(workgroup_size);

    let output = cx.bind_by(&output);
    let global_id = cx.global_invocation_id().x();

    output.index(global_id).store(global_id);
    cx.create_compute_pipeline(&gpu).unwrap()
  };

  for _ in 0..50 {
    let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
      BindingBuilder::default()
        .with_bind(&output)
        .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
      pass.dispatch_workgroups(1, 1, 1);
    });
    let result = encoder.read_buffer(&gpu.device, &output);

    gpu.submit_encoder(encoder);
    let submit_time = std::time::Instant::now();

    let _ = result.await.unwrap();
    let get_result_time = std::time::Instant::now();

    println!("latency: {:?}", get_result_time - submit_time);
  }
}
