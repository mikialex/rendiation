use interphaser::*;
use reactive::once_forever_pending;

use crate::{button, checkbox, terminal, Viewer};

pub fn viewer() -> impl View {
  let (terminal, to_execute) = terminal();

  let viewer = Viewer::new(to_execute);

  let (button_view, _button_action) = button(String::from("test button"));
  let (checkbox_view, _) = checkbox(once_forever_pending(true));

  // // how do I do log??
  // Box::leak(Box::new(button_action));

  AbsoluteAnchor::default().wrap(
    absolute_group()
      .child(AbsChild::new(GPUCanvas::new(viewer)))
      .child(AbsChild::new(terminal).with_position((0., 0.)))
      .child(AbsChild::new(button_view).with_position((500., 500.)))
      .child(AbsChild::new(checkbox_view).with_position((500., 600.))),
    // .child(AbsChild::new(perf_panel()).with_position((0., 50.))),
  )
}

// fn perf_panel() -> impl Component {
//   Container::sized((500., 200.))
//     .padding(RectBoundaryWidth::equal(5.))
//     .nest_over(
//       Text::default().with_layout(TextLayoutConfig::SizedBox {
//         line_wrap: LineWrap::Multiple,
//         horizon_align: TextHorizontalAlignment::Left,
//         vertical_align: TextVerticalAlignment::Top,
//       }), /* .bind_with_ctx(|s, _t, ctx| {
//            * let content = format!(
//            * "frame_id: {}\nupdate_time: {}\nlayout_time: {}\nrendering_prepare_time:
//            * {}\nrendering_dispatch_time: {}",     ctx.last_frame_perf_info.frame_id,
//            * ctx.last_frame_perf_info.update_time.as_micros() as f32 / 1000.,
//            * ctx.last_frame_perf_info.layout_time.as_micros() as f32 / 1000.,
//            * ctx.last_frame_perf_info.rendering_prepare_time.as_micros() as f32 / 1000.,
//            * ctx.last_frame_perf_info.rendering_dispatch_time.as_micros() as f32 / 1000.,
//            * );
//            * s.content.set(content);
//            * }) */
//     )
// }
