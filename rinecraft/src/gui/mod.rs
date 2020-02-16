use rendium::*;

struct RinecraftUI {
  debug_hud: DebugHUD,
  view: RinecraftPage,
}

impl Component<Self> for RinecraftUI {
  fn render(&self) -> ComponentTree<Self> {
    let field_lens = Field::new(
      |s: &RinecraftUI| &s.debug_hud,
      |s: &mut RinecraftUI| &mut s.debug_hud,
    );
    // match self.view {
    //   Loading => {

    //   }
    //   _ => todo!()
    // }
    let hud: LensWrap<RinecraftUI, _, _> = LensWrap::new(DebugHUDComponent{}, field_lens);
    todo!()
  }
}

enum RinecraftPage {
  Loading { percentage: f32 },
  NewWorld {},
}

struct DebugHUD {
  fps: f32
}

struct DebugHUDComponent{}

impl Component<DebugHUD> for DebugHUDComponent {
  fn render(&self) -> ComponentTree<DebugHUD> {
    todo!()
  }
}
