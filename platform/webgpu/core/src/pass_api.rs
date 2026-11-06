//! The wgpu pass api wrapper, the calls are counted by the pass itself, see [GPUCommandCallCounts].

use crate::*;

impl GPURenderPass {
  /// the call counts recorded in this pass, they are merged into the encoder when the pass dropped
  pub fn call_counts(&self) -> &GPUCommandCallCounts {
    &self.counts
  }

  /// Access the underlying wgpu render pass.
  ///
  /// The calls made through the raw pass are not counted in [GPUCommandCallCounts]. Only use it
  /// when the wgpu type is required, for example to integrate the third party renderer. Any common
  /// usage should use or add the wrapper method in [GPURenderPass].
  pub fn raw_pass(&mut self) -> &mut gpu::RenderPass<'static> {
    &mut self.pass
  }

  pub fn set_bind_group<'a, BG>(&mut self, index: u32, bind_group: BG, offsets: &[DynamicOffset])
  where
    Option<&'a gpu::BindGroup>: From<BG>,
  {
    self.counts.set_bind_group += 1;
    self.pass.set_bind_group(index, bind_group, offsets);
  }

  pub fn set_pipeline(&mut self, pipeline: &gpu::RenderPipeline) {
    self.counts.set_pipeline += 1;
    self.pass.set_pipeline(pipeline);
  }

  pub fn set_blend_constant(&mut self, color: Color) {
    self.counts.set_blend_constant += 1;
    self.pass.set_blend_constant(color);
  }

  pub fn set_index_buffer(&mut self, buffer_slice: gpu::BufferSlice, index_format: IndexFormat) {
    self.counts.set_index_buffer += 1;
    self.pass.set_index_buffer(buffer_slice, index_format);
  }

  pub fn set_vertex_buffer(&mut self, slot: u32, buffer_slice: gpu::BufferSlice) {
    self.counts.set_vertex_buffer += 1;
    self.pass.set_vertex_buffer(slot, buffer_slice);
  }

  pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
    self.counts.set_scissor_rect += 1;
    self.pass.set_scissor_rect(x, y, width, height);
  }

  pub fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
    self.counts.set_viewport += 1;
    self.pass.set_viewport(x, y, w, h, min_depth, max_depth);
  }

  pub fn set_stencil_reference(&mut self, reference: u32) {
    self.counts.set_stencil_reference += 1;
    self.pass.set_stencil_reference(reference);
  }

  pub fn set_immediates(&mut self, offset: u32, data: &[u8]) {
    self.counts.set_immediates += 1;
    self.counts.set_immediates_bytes += data.len() as u64;
    self.pass.set_immediates(offset, data);
  }

  pub fn insert_debug_marker(&mut self, label: &str) {
    self.counts.debug_marker += 1;
    self.pass.insert_debug_marker(label);
  }

  pub fn push_debug_group(&mut self, label: &str) {
    self.counts.debug_marker += 1;
    self.pass.push_debug_group(label);
  }

  pub fn pop_debug_group(&mut self) {
    self.counts.debug_marker += 1;
    self.pass.pop_debug_group();
  }

  pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
    self.counts.draw += 1;
    self.pass.draw(vertices, instances);
  }

  pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
    self.counts.draw_indexed += 1;
    self.pass.draw_indexed(indices, base_vertex, instances);
  }

  pub fn draw_mesh_tasks(&mut self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
    self.counts.draw_mesh_tasks += 1;
    self
      .pass
      .draw_mesh_tasks(group_count_x, group_count_y, group_count_z);
  }

  pub fn draw_indirect(&mut self, indirect_buffer: &gpu::Buffer, indirect_offset: u64) {
    self.counts.draw_indirect += 1;
    self.pass.draw_indirect(indirect_buffer, indirect_offset);
  }

  pub fn draw_indexed_indirect(&mut self, indirect_buffer: &gpu::Buffer, indirect_offset: u64) {
    self.counts.draw_indexed_indirect += 1;
    self
      .pass
      .draw_indexed_indirect(indirect_buffer, indirect_offset);
  }

  pub fn draw_mesh_tasks_indirect(&mut self, indirect_buffer: &gpu::Buffer, indirect_offset: u64) {
    self.counts.draw_mesh_tasks_indirect += 1;
    self
      .pass
      .draw_mesh_tasks_indirect(indirect_buffer, indirect_offset);
  }

  pub fn multi_draw_indirect(
    &mut self,
    indirect_buffer: &gpu::Buffer,
    indirect_offset: u64,
    count: u32,
  ) {
    self.counts.multi_draw_indirect += 1;
    self.counts.multi_draw_max_count += count as u64;
    self
      .pass
      .multi_draw_indirect(indirect_buffer, indirect_offset, count);
  }

  pub fn multi_draw_indexed_indirect(
    &mut self,
    indirect_buffer: &gpu::Buffer,
    indirect_offset: u64,
    count: u32,
  ) {
    self.counts.multi_draw_indexed_indirect += 1;
    self.counts.multi_draw_max_count += count as u64;
    self
      .pass
      .multi_draw_indexed_indirect(indirect_buffer, indirect_offset, count);
  }

  pub fn multi_draw_mesh_tasks_indirect(
    &mut self,
    indirect_buffer: &gpu::Buffer,
    indirect_offset: u64,
    count: u32,
  ) {
    self.counts.multi_draw_mesh_tasks_indirect += 1;
    self.counts.multi_draw_max_count += count as u64;
    self
      .pass
      .multi_draw_mesh_tasks_indirect(indirect_buffer, indirect_offset, count);
  }

  pub fn multi_draw_indirect_count(
    &mut self,
    indirect_buffer: &gpu::Buffer,
    indirect_offset: u64,
    count_buffer: &gpu::Buffer,
    count_offset: u64,
    max_count: u32,
  ) {
    self.counts.multi_draw_indirect_count += 1;
    self.counts.multi_draw_max_count += max_count as u64;
    self.pass.multi_draw_indirect_count(
      indirect_buffer,
      indirect_offset,
      count_buffer,
      count_offset,
      max_count,
    );
  }

  pub fn multi_draw_indexed_indirect_count(
    &mut self,
    indirect_buffer: &gpu::Buffer,
    indirect_offset: u64,
    count_buffer: &gpu::Buffer,
    count_offset: u64,
    max_count: u32,
  ) {
    self.counts.multi_draw_indexed_indirect_count += 1;
    self.counts.multi_draw_max_count += max_count as u64;
    self.pass.multi_draw_indexed_indirect_count(
      indirect_buffer,
      indirect_offset,
      count_buffer,
      count_offset,
      max_count,
    );
  }

  pub fn multi_draw_mesh_tasks_indirect_count(
    &mut self,
    indirect_buffer: &gpu::Buffer,
    indirect_offset: u64,
    count_buffer: &gpu::Buffer,
    count_offset: u64,
    max_count: u32,
  ) {
    self.counts.multi_draw_mesh_tasks_indirect_count += 1;
    self.counts.multi_draw_max_count += max_count as u64;
    self.pass.multi_draw_mesh_tasks_indirect_count(
      indirect_buffer,
      indirect_offset,
      count_buffer,
      count_offset,
      max_count,
    );
  }

  pub fn execute_bundles<'a, I: IntoIterator<Item = &'a gpu::RenderBundle>>(
    &mut self,
    render_bundles: I,
  ) {
    self.counts.execute_bundles += 1;
    self.pass.execute_bundles(render_bundles);
  }

  pub fn write_timestamp(&mut self, query_set: &gpu::QuerySet, query_index: u32) {
    self.counts.write_timestamp += 1;
    self.pass.write_timestamp(query_set, query_index);
  }

  pub fn begin_occlusion_query(&mut self, query_index: u32) {
    self.counts.begin_occlusion_query += 1;
    self.pass.begin_occlusion_query(query_index);
  }

  pub fn end_occlusion_query(&mut self) {
    self.counts.end_occlusion_query += 1;
    self.pass.end_occlusion_query();
  }

  pub fn begin_pipeline_statistics_query(&mut self, query_set: &gpu::QuerySet, query_index: u32) {
    self.counts.begin_pipeline_statistics_query += 1;
    self
      .pass
      .begin_pipeline_statistics_query(query_set, query_index);
  }

  pub fn end_pipeline_statistics_query(&mut self) {
    self.counts.end_pipeline_statistics_query += 1;
    self.pass.end_pipeline_statistics_query();
  }
}

impl GPUComputePass {
  /// the call counts recorded in this pass, they are merged into the encoder when the pass dropped
  pub fn call_counts(&self) -> &GPUCommandCallCounts {
    &self.counts
  }

  /// Access the underlying wgpu compute pass.
  ///
  /// The calls made through the raw pass are not counted in [GPUCommandCallCounts]. Only use it
  /// when the wgpu type is required, for example to integrate the third party renderer. Any common
  /// usage should use or add the wrapper method in [GPUComputePass].
  pub fn raw_pass(&mut self) -> &mut gpu::ComputePass<'static> {
    &mut self.pass
  }

  pub fn set_bind_group<'a, BG>(&mut self, index: u32, bind_group: BG, offsets: &[DynamicOffset])
  where
    Option<&'a gpu::BindGroup>: From<BG>,
  {
    self.counts.set_bind_group += 1;
    self.pass.set_bind_group(index, bind_group, offsets);
  }

  pub fn set_pipeline(&mut self, pipeline: &gpu::ComputePipeline) {
    self.counts.set_pipeline += 1;
    self.pass.set_pipeline(pipeline);
  }

  pub fn set_immediates(&mut self, offset: u32, data: &[u8]) {
    self.counts.set_immediates += 1;
    self.counts.set_immediates_bytes += data.len() as u64;
    self.pass.set_immediates(offset, data);
  }

  pub fn insert_debug_marker(&mut self, label: &str) {
    self.counts.debug_marker += 1;
    self.pass.insert_debug_marker(label);
  }

  pub fn push_debug_group(&mut self, label: &str) {
    self.counts.debug_marker += 1;
    self.pass.push_debug_group(label);
  }

  pub fn pop_debug_group(&mut self) {
    self.counts.debug_marker += 1;
    self.pass.pop_debug_group();
  }

  pub fn dispatch_workgroups(&mut self, x: u32, y: u32, z: u32) {
    self.counts.dispatch_workgroups += 1;
    self.pass.dispatch_workgroups(x, y, z);
  }

  pub fn dispatch_workgroups_indirect(
    &mut self,
    indirect_buffer: &gpu::Buffer,
    indirect_offset: u64,
  ) {
    self.counts.dispatch_workgroups_indirect += 1;
    self
      .pass
      .dispatch_workgroups_indirect(indirect_buffer, indirect_offset);
  }

  pub fn write_timestamp(&mut self, query_set: &gpu::QuerySet, query_index: u32) {
    self.counts.write_timestamp += 1;
    self.pass.write_timestamp(query_set, query_index);
  }

  pub fn begin_pipeline_statistics_query(&mut self, query_set: &gpu::QuerySet, query_index: u32) {
    self.counts.begin_pipeline_statistics_query += 1;
    self
      .pass
      .begin_pipeline_statistics_query(query_set, query_index);
  }

  pub fn end_pipeline_statistics_query(&mut self) {
    self.counts.end_pipeline_statistics_query += 1;
    self.pass.end_pipeline_statistics_query();
  }

  pub fn map_buffer_on_submit<S: core::ops::RangeBounds<gpu::BufferAddress>>(
    &mut self,
    buffer: &gpu::Buffer,
    mode: gpu::MapMode,
    bounds: S,
    callback: impl FnOnce(Result<(), gpu::BufferAsyncError>) + gpu::WasmNotSend + 'static,
  ) {
    self.counts.map_buffer_on_submit += 1;
    self
      .pass
      .map_buffer_on_submit(buffer, mode, bounds, callback);
  }

  pub fn on_submitted_work_done(&mut self, callback: impl FnOnce() + Send + 'static) {
    self.counts.on_submitted_work_done += 1;
    self.pass.on_submitted_work_done(callback);
  }
}
