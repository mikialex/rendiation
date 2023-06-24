use interphaser::*;

use crate::{menu, terminal, MenuList, MenuModel, ViewerImpl};

pub struct ViewerApplication {
  pub menu: MenuModel,
  pub viewer: ViewerImpl,
}

impl Default for ViewerApplication {
  fn default() -> Self {
    ViewerApplication {
      viewer: Default::default(),
      menu: create_menu(),
    }
  }
}

pub fn create_app() -> impl UIComponent<ViewerApplication> {
  Flex::column().wrap(
    flex_group()
      .child(Child::fixed(menu().lens(lens!(ViewerApplication, menu))))
      .child(Child::flex(
        viewer().lens(lens!(ViewerApplication, viewer)),
        1.,
      )),
  )
}

pub fn viewer() -> impl UIComponent<ViewerImpl> {
  AbsoluteAnchor::default().wrap(
    absolute_group()
      .child(AbsChild::new(GPUCanvas::default()))
      .child(AbsChild::new(terminal().lens(lens!(ViewerImpl, terminal))).with_position((0., 0.)))
      .child(AbsChild::new(perf_panel()).with_position((0., 50.))),
  )
}

fn create_menu() -> MenuModel {
  MenuModel {
    lists: vec![
      MenuList {
        name: "3D Examples".to_string(),
        items: Vec::new(),
      },
      MenuList {
        name: "UI Examples".to_string(),
        items: Vec::new(),
      },
    ],
  }
}

fn perf_panel<T: 'static>() -> impl UIComponent<T> {
  Container::sized((500., 200.))
    .padding(QuadBoundaryWidth::equal(5.))
    .wrap(
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
