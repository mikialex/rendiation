use interphaser::*;

use crate::Viewer;

pub fn create_app() -> impl UIComponent<Viewer> {
  AbsoluteAnchor::default().wrap(
    absolute_group()
      .child(AbsChild::new(
        GPUCanvas::default().lens(lens!(Viewer, viewer)),
      ))
      // .child(AbsChild::new(build_todo().lens(lens!(Viewer, todo))))
      .child(AbsChild::new(perf_panel())),
  )
}

pub fn perf_panel<T: 'static>() -> impl UIComponent<T> {
  Container::size((500., 200.)).wrap(
    Text::default()
    .with_line_wrap(LineWrap::Multiple)
    .with_horizon_align(HorizontalAlign::Left)
    .with_vertical_align(VerticalAlign::Top)
    .bind_with_ctx(|s, _t, ctx| {
      let content = format!(
        "frame_id: {}\nupdate_time: {}\nlayout_time: {}\nrendering_prepare_time: {}\nrendering_dispatch_time: {}",
        ctx.last_frame_perf_info.frame_id,
        ctx.last_frame_perf_info.update_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.layout_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.rendering_prepare_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.rendering_dispatch_time.as_micros() as f32 / 1000.,
      );
      s.content.set(content);
    })
  )
}
