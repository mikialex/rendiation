use rendium::*;

struct RinecraftUI {
  debug_hud: DebugHUD,
  view: RinecraftPage,
}

impl Component<Self> for RinecraftUI {
  fn render(&self) -> ComponentTree<Self> {
    // match self.view {
    //   Loading => {

    //   }
    //   _ => todo!()
    // }
    todo!()
  }
}

enum RinecraftPage {
  Loading { percentage: f32 },
  NewWorld {},
}

struct DebugHUD{

}