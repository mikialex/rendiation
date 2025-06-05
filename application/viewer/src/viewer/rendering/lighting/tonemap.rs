use rendiation_texture_gpu_process::*;

use crate::*;

pub fn use_tonemap<'a, 'b>(
  cx: &'a mut Viewer3dRenderingCx<'b>,
) -> (&'a mut Viewer3dRenderingCx<'b>, &'b ToneMap) {
  let (cx, tonemap) = cx.use_gpu_state(ToneMap::new);

  cx.on_gui(|ui| {
    if is_hdr_rendering {
      ui.label("tonemap is disabled when hdr display enabled");
      tonemap.ty = ToneMapType::None;
    } else {
      if tonemap.ty == ToneMapType::None {
        tonemap.ty = ToneMapType::ACESFilmic;
      }
      egui::ComboBox::from_label("Tone mapping type")
        .selected_text(format!("{:?}", &tonemap.ty))
        .show_ui(ui, |ui| {
          ui.selectable_value(&mut tonemap.ty, ToneMapType::Linear, "Linear");
          ui.selectable_value(&mut tonemap.ty, ToneMapType::Cineon, "Cineon");
          ui.selectable_value(&mut tonemap.ty, ToneMapType::Reinhard, "Reinhard");
          ui.selectable_value(&mut tonemap.ty, ToneMapType::ACESFilmic, "ACESFilmic");
        });

      tonemap.mutate_exposure(|e| {
        ui.add(
          egui::Slider::new(e, 0.0..=2.0)
            .step_by(0.05)
            .text("exposure"),
        );
      });
    }
  });

  cx.on_render(|frame_ctx, _| {
    tonemap.update(frame_ctx.gpu);
  });

  (cx, tonemap)
}
