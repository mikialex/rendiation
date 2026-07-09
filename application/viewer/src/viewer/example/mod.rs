pub use text3d::text3d_content_edit_ui;
use text3d::use_text3d_example;

mod text3d;
mod texture_material_share;
mod transform_instance;
mod util;

pub use texture_material_share::*;
pub use transform_instance::*;
pub use util::*;

use crate::*;

#[derive(Default)]
struct ExampleRegistry {
  examples: FastHashMap<String, Box<dyn Fn(&mut ViewerCx)>>,
  current_active: Option<String>,
}

impl ExampleRegistry {
  pub fn register(&mut self, name: &str, f: impl Fn(&mut ViewerCx) + 'static) {
    self.examples.insert(name.to_string(), Box::new(f));
  }
}

pub fn use_viewer_examples(cx: &mut ViewerCx) {
  cx.next_key_scope_root();
  let (cx, registry) = cx.use_plain_state_init(|_| {
    let mut registry = ExampleRegistry::default();
    registry.register("Text3d example", use_text3d_example);
    registry.register(
      "Texture and Material Share",
      use_texture_material_share_example,
    );
    registry.register("Transform Instance Example", use_transform_instance_example);
    registry
  });

  if let ViewerCxStage::Gui {
    egui_ctx, global, ..
  } = &mut cx.stage
  {
    let opened = global.features.entry("examples").or_insert(false);

    egui::Window::new("Examples")
      .vscroll(true)
      .open(opened)
      .show(egui_ctx, |ui| {
        //
        egui::ComboBox::from_label("lists")
          .selected_text(format!("{:?}", &registry.current_active))
          .show_ui(ui, |ui| {
            ui.selectable_value(&mut registry.current_active, None, "None");
            for (name, _) in &registry.examples {
              ui.selectable_value(&mut registry.current_active, Some(name.clone()), name);
            }
          });
      });
  }

  cx.next_key_scope_root();
  if let Some(active) = &registry.current_active {
    if let Some(f) = registry.examples.get(active) {
      cx.keyed_scope(active, |cx| {
        f(cx);
      })
    } else {
      log::error!("unknown active example: {active}")
    }
  }
  //
}
