use rand::Rng;

use super::util::SceneModelWithUniqueNode;
use crate::*;

pub fn use_text3d_example(cx: &mut ViewerCx) {
  let (cx, example) = cx.use_state_init(|_| Text3DExample::new());

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    let mut text3d_writer = global_entity_of::<Text3dEntity>().entity_writer();

    // process deletions
    if !example.pending_deletions.is_empty() {
      while let Some(inst) = example.pending_deletions.pop() {
        inst.destroy(writer, &mut text3d_writer);
      }
    }

    // process additions
    if !example.pending_additions.is_empty() {
      while let Some(info) = example.pending_additions.pop() {
        example.create_instance(writer, &mut text3d_writer, cx.default_scene.scene, info);
      }
    }
  }

  if let ViewerCxStage::Gui { egui_ctx, .. } = &mut cx.stage {
    egui::Window::new("Text3d example")
      .default_size((300., 600.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        ui.heading("new text content:");
        text3d_content_edit_ui(ui, &mut example.next_text_to_add);

        if ui
          .button("set predefined text: multi_line_with_chinese")
          .clicked()
        {
          example.next_text_to_add.content = "你好!\nHello world!\nこんにちは!".to_string();
        }

        ui.separator();

        ui.horizontal(|ui| {
          if ui.button("add text").clicked() {
            example
              .pending_additions
              .push(example.next_text_to_add.clone());
          }
          if ui.button("clear all").clicked() {
            example.pending_deletions.extend(example.instance.drain(..));
          }
        });

        ui.separator();
        ui.heading("all text instances:");
        egui::ScrollArea::vertical()
          .max_height(300.)
          .show(ui, |ui| {
            // collect which instances to remove after rendering
            let mut to_remove = Vec::new();
            for (idx, inst) in example.instance.iter().enumerate() {
              if let Some(Some(content)) =
                get_db_view::<Text3dContent>().access(inst.text.raw_handle_ref())
              {
                ui.horizontal(|ui| {
                  ui.label(format!(
                    "id:{} | \"{}\"",
                    inst.text,
                    content.content.chars().take(30).collect::<String>()
                  ));

                  if ui.button("layout_info").clicked() {
                    if let Some(r) = compute_text_layout_info(
                      inst.text.into_raw(),
                      &mut cx.viewer.font_system.write(),
                    ) {
                      log::info!("{:?}", r);
                    } else {
                      log::error!("failed to compute layout info");
                    }
                  }
                  if ui.button("🗑").clicked() {
                    to_remove.push(idx);
                  }
                });
              }
            }
            // remove in reverse to keep indices valid
            for idx in to_remove.into_iter().rev() {
              let inst = example.instance.remove(idx);
              example.pending_deletions.push(inst);
            }
          });
      });
  }
}

struct Text3DExample {
  instance: Vec<Text3DTestInstance>,
  next_text_to_add: Text3dContentInfo,
  pending_additions: Vec<Text3dContentInfo>,
  pending_deletions: Vec<Text3DTestInstance>,
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for Text3DExample {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    self.destroy(&mut cx.writer, &mut global_entity_of().entity_writer());
  }
}

impl Text3DExample {
  pub fn new() -> Self {
    Self {
      instance: Vec::new(),
      pending_additions: Vec::new(),
      pending_deletions: Vec::new(),
      next_text_to_add: Text3dContentInfo {
        content: String::from("Hello abcd!"),
        font_size: 12.,
        line_height: 1.2,
        font: Some(String::from("Arial")),
        weight: None,
        color: Vec4::new(1., 0., 0., 1.),
        italic: false,
        width: None,
        height: None,
        align: TextAlignment::Left,
        underline: false,
      },
    }
  }

  fn create_instance(
    &mut self,
    writer: &mut SceneWriter,
    text3d_writer: &mut TableWriter<Text3dEntity>,
    scene: EntityHandle<SceneEntity>,
    info: Text3dContentInfo,
  ) {
    let mut rng = rand::rng();
    let x = rng.random_range(-2.0f64..2.0);
    let y = rng.random_range(-2.0f64..2.0);
    let z = rng.random_range(-2.0f64..2.0);

    let text = text3d_writer.new_entity(|w| {
      w.write::<Text3dContent>(&Some(ExternalRefPtr::new(info)))
        .write::<Text3dLocalTransform>(&Mat4::scale((0.05, 0.05, 0.05)))
    });

    let child = writer.create_root_child();
    writer.set_local_matrix(child, Mat4::translate((x, y, z)));

    let scene = scene.some_handle();
    let model = writer.model_writer.new_entity(|w| {
      w.write::<SceneModelText3dPayload>(&text.some_handle())
        .write::<SceneModelBelongsToScene>(&scene)
        .write::<SceneModelRefNode>(&child.some_handle())
    });

    self.instance.push(Text3DTestInstance {
      text,
      scene_unit: SceneModelWithUniqueNode { model, node: child },
    });
  }

  pub fn destroy(
    &mut self,
    writer: &mut SceneWriter,
    text3d_writer: &mut TableWriter<Text3dEntity>,
  ) {
    for instance in self.instance.drain(..) {
      instance.destroy(writer, text3d_writer);
    }
    for instance in self.pending_deletions.drain(..) {
      instance.destroy(writer, text3d_writer);
    }
  }
}

struct Text3DTestInstance {
  text: EntityHandle<Text3dEntity>,
  scene_unit: SceneModelWithUniqueNode,
}

impl Text3DTestInstance {
  pub fn destroy(self, writer: &mut SceneWriter, text3d_writer: &mut TableWriter<Text3dEntity>) {
    text3d_writer.delete_entity(self.text);
    self.scene_unit.destroy(writer);
  }
}

pub fn text3d_content_edit_ui(ui: &mut egui::Ui, c: &mut Text3dContentInfo) {
  ui.text_edit_multiline(&mut c.content);
  ui.checkbox(&mut c.italic, "italic");
  ui.checkbox(&mut c.underline, "underline");

  let mut has_width = c.width.is_some();
  ui.checkbox(&mut has_width, "enable_width");
  if has_width {
    if c.width.is_none() {
      c.width = Some(100.);
    }
  } else {
    c.width = None;
  }
  if let Some(width) = &mut c.width {
    ui.add(egui::Slider::new(width, 0.0..=200.0).text("width"));
  }

  let mut has_height = c.height.is_some();
  ui.checkbox(&mut has_height, "enable_height");
  if has_height {
    if c.height.is_none() {
      c.height = Some(100.);
    }
  } else {
    c.height = None;
  }
  if let Some(height) = &mut c.height {
    ui.add(egui::Slider::new(height, 0.0..=200.0).text("height"));
  }

  ui.add(egui::Slider::new(&mut c.font_size, 4.0..=128.0).text("font_size"));
  ui.add(egui::Slider::new(&mut c.line_height, 0.5..=3.0).text("line_height"));
  let mut rgba = [c.color.x, c.color.y, c.color.z, c.color.w];
  ui.color_edit_button_rgba_unmultiplied(&mut rgba);
  c.color = Vec4::new(rgba[0], rgba[1], rgba[2], rgba[3]);

  egui::ComboBox::from_label("alignment")
    .selected_text(format!("{:?}", &c.align))
    .show_ui(ui, |ui| {
      ui.selectable_value(&mut c.align, TextAlignment::Left, "left");
      ui.selectable_value(&mut c.align, TextAlignment::Center, "Center");
      ui.selectable_value(&mut c.align, TextAlignment::Right, "Right");
    });
}
