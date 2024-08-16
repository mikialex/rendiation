use rendiation_shader_api::*;
use rendiation_webgpu::*;

#[pollster::main]
pub async fn main() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let workgroup_size: u32 = 64;

  let input_data = vec![1_u32; workgroup_size as usize]; // here we only demo workgroup case..
  let input = create_gpu_readonly_storage(input_data.as_slice(), &gpu);
  let output = create_gpu_read_write_storage::<[u32]>(input_data.len(), &gpu);

  let pipeline = {
    let mut cx = compute_shader_builder().with_config_work_group_size(workgroup_size);
    // .with_log_shader();

    let shared = cx.define_workgroup_shared_var_host_size_array::<u32>(workgroup_size);
    let input = cx.bind_by(&input);
    let output = cx.bind_by(&output);

    let global_id = cx.global_invocation_id().x();
    let local_id = cx.local_invocation_id().x();

    let value = input.index(global_id).load().make_local_var();

    shared.index(local_id).store(value.load());

    workgroup_size.ilog2().into_shader_iter().for_each(|i, _| {
      workgroup_barrier();

      if_by(local_id.greater_equal_than(val(1) << i), || {
        value.store(value.load() + shared.index(local_id - (val(1) << i)).load())
      });

      workgroup_barrier();
      shared.index(local_id).store(value.load())
    });

    output.index(global_id).store(value.load());

    cx.create_compute_pipeline(&gpu).unwrap()
  };

  let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::new_as_compute()
      .with_bind(&input)
      .with_bind(&output)
      .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(1, 1, 1);
  });
  let result = encoder.read_buffer(&gpu.device, &output);
  gpu.submit_encoder(encoder);

  let result = result.await.unwrap();
  let result = <[u32]>::from_bytes_into_boxed(&result.read_raw()).into_vec();
  println!("{:?}", result);
}
