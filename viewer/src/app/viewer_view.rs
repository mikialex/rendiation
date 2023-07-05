use interphaser::*;

use crate::{terminal, ViewerImpl};

pub struct ViewerApplication {
  pub viewer: ViewerImpl,
}

impl Default for ViewerApplication {
  fn default() -> Self {
    ViewerApplication {
      viewer: Default::default(),
    }
  }
}

pub fn create_app() -> impl Component {
  Flex::column().nest_over(flex_group().child(Child::flex(viewer(), 1.)))
}

pub fn viewer() -> impl Component {
  AbsoluteAnchor::default().wrap(
    absolute_group()
      .child(AbsChild::new(GPUCanvas::default()))
      .child(AbsChild::new(terminal().with_position((0., 0.))))
      .child(AbsChild::new(perf_panel()).with_position((0., 50.))),
  )
}

fn perf_panel() -> impl Component {
  Container::sized((500., 200.))
    .padding(RectBoundaryWidth::equal(5.))
    .nest_over(
    Text::default()
    .with_layout(TextLayoutConfig::SizedBox{
        line_wrap: LineWrap::Multiple,
        horizon_align: TextHorizontalAlignment::Left,
        vertical_align: TextVerticalAlignment::Top,
    })
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
