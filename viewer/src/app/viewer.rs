use interphaser::*;

use crate::{menu, MenuList, MenuModel, UIExamples, ViewerImpl};

pub struct ViewerApplication {
  pub ui_examples: UIExamples,
  pub menu: MenuModel,
  pub viewer: ViewerImpl,
}

impl Default for ViewerApplication {
  fn default() -> Self {
    ViewerApplication {
      ui_examples: Default::default(),
      viewer: Default::default(),
      menu: create_menu(),
    }
  }
}

pub fn create_app() -> impl UIComponent<ViewerApplication> {
  Flex::column().wrap(
    flex_group()
      .child(Child::flex(menu().lens(lens!(ViewerApplication, menu)), 1.))
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
      // .child(AbsChild::new(build_todo().lens(lens!(Viewer, todo))))
      .child(AbsChild::new(perf_panel())),
  )
}

fn create_menu() -> MenuModel {
  MenuModel {
    lists: vec![
      MenuList {
        name: "File".to_string(),
        items: Vec::new(),
      },
      MenuList {
        name: "Edit".to_string(),
        items: Vec::new(),
      },
      MenuList {
        name: "Settings".to_string(),
        items: Vec::new(),
      },
      MenuList {
        name: "Help".to_string(),
        items: Vec::new(),
      },
    ],
  }
}

fn perf_panel<T: 'static>() -> impl UIComponent<T> {
  Container::size((500., 200.)).wrap(
    Text::default()
    .with_line_wrap(LineWrap::Multiple)
    .with_horizon_align(HorizontalAlignment::Left)
    .with_vertical_align(VerticalAlignment::Top)
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
