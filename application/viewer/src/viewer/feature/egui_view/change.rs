use egui::*;
use rendiation_algebra::*;

pub struct UiWithChangeInfo<'a>(pub &'a mut Ui, pub bool);
impl<'x> UiWithChangeInfo<'x> {
  pub fn separator(&mut self) -> Response {
    let res = self.0.separator();
    self.1 |= res.changed();
    res
  }

  pub fn button<'a>(&mut self, atoms: impl IntoAtoms<'a>) -> Response {
    let res = self.0.button(atoms);
    self.1 |= res.changed();
    res
  }

  pub fn selectable_value<'a, Value: PartialEq>(
    &mut self,
    current_value: &mut Value,
    selected_value: Value,
    text: impl IntoAtoms<'a>,
  ) -> Response {
    let res = self.0.selectable_value(current_value, selected_value, text);
    self.1 |= res.changed();
    res
  }

  pub fn add(&mut self, widget: impl Widget) -> Response {
    let res = self.0.add(widget);
    self.1 |= res.changed();
    res
  }

  pub fn add_enabled_ui<R>(
    &mut self,
    enabled: bool,
    add_contents: impl FnOnce(&mut UiWithChangeInfo) -> R,
  ) -> InnerResponse<R> {
    let res = self.0.add_enabled_ui(enabled, |rui| {
      let mut u = UiWithChangeInfo(rui, false);
      let r = add_contents(&mut u);
      self.1 |= u.1;
      r
    });
    self.1 |= res.response.changed();
    res
  }

  pub fn checkbox<'a>(&mut self, checked: &'a mut bool, atoms: impl IntoAtoms<'a>) -> Response {
    let res = self.0.checkbox(checked, atoms);
    self.1 |= res.changed();
    res
  }
  pub fn label(&mut self, text: impl Into<WidgetText>) -> Response {
    self.0.label(text)
  }
  pub fn collapsing<R>(
    &mut self,
    heading: impl Into<WidgetText>,
    add_contents: impl FnOnce(&mut UiWithChangeInfo) -> R,
  ) -> CollapsingResponse<R> {
    self.0.collapsing(heading, |ui| {
      let mut ui = UiWithChangeInfo(ui, false);
      let res = add_contents(&mut ui);
      self.1 |= ui.1;
      res
    })
  }
  pub fn color_edit_button_rgba_unmultiplied(&mut self, rgba_unmul: &mut [f32; 4]) -> Response {
    let res = self.0.color_edit_button_rgba_unmultiplied(rgba_unmul);
    self.1 |= res.changed();
    res
  }
  pub fn color_edit_button_rgb(&mut self, rgb: &mut [f32; 3]) -> Response {
    let res = self.0.color_edit_button_rgb(rgb);
    self.1 |= res.changed();
    res
  }
}

pub fn modify_color4_change(ui: &mut UiWithChangeInfo, c: &mut Vec4<f32>) {
  let mut color: [f32; 4] = (*c).into();
  ui.color_edit_button_rgba_unmultiplied(&mut color);
  *c = color.into();
}

pub fn modify_color_change(ui: &mut UiWithChangeInfo, c: &mut Vec3<f32>) {
  let mut color: [f32; 3] = (*c).into();
  ui.color_edit_button_rgb(&mut color);
  *c = color.into();
}

pub trait ComboEguiExt {
  fn show_ui_changed<R>(
    self,
    ui: &mut UiWithChangeInfo,
    menu_contents: impl FnOnce(&mut UiWithChangeInfo) -> R,
  ) -> InnerResponse<Option<R>>;
}

impl ComboEguiExt for ComboBox {
  fn show_ui_changed<R>(
    self,
    ui: &mut UiWithChangeInfo,
    menu_contents: impl FnOnce(&mut UiWithChangeInfo) -> R,
  ) -> InnerResponse<Option<R>> {
    self.show_ui(ui.0, |rui| {
      let mut u = UiWithChangeInfo(rui, false);
      let r = menu_contents(&mut u);
      ui.1 |= u.1;
      r
    })
  }
}
