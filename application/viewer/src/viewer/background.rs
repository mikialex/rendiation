use crate::*;

#[derive(Debug, PartialEq, Clone, Copy)]
enum ViewerBackgroundType {
  Color,
  Environment,
  //   Sky,
}

pub struct ViewerBackgroundState {
  current: ViewerBackgroundType,
  default_env_background: EntityHandle<SceneTextureCubeEntity>,
  solid_background_color: [f32; 3],
}

const DEFAULT_BACKGROUND: Vec3<f32> = Vec3::new(0.8, 0.8, 0.8);

impl ViewerBackgroundState {
  pub fn init(writer: &mut SceneWriter) -> Self {
    writer.set_solid_background(DEFAULT_BACKGROUND);
    let default_env_background = load_example_cube_tex(writer);
    Self {
      current: ViewerBackgroundType::Color,
      default_env_background,
      solid_background_color: DEFAULT_BACKGROUND.into(),
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui, scene: EntityHandle<SceneEntity>) {
    ui.collapsing("Background", |ui| {
      let previous = self.current;
      egui::ComboBox::from_label("background type")
        .selected_text(format!("{:?}", &self.current))
        .show_ui(ui, |ui| {
          ui.selectable_value(
            &mut self.current,
            ViewerBackgroundType::Color,
            "Solid Color",
          );
          ui.selectable_value(
            &mut self.current,
            ViewerBackgroundType::Environment,
            "EnvBackground",
          );
        });

      {
        let mut writer = SceneWriter::from_global(scene);
        match self.current {
          ViewerBackgroundType::Color => {
            ui.color_edit_button_rgb(&mut self.solid_background_color);
            writer.set_solid_background(Vec3::from(self.solid_background_color))
          }
          ViewerBackgroundType::Environment => {}
        }
        if self.current != previous {
          match self.current {
            ViewerBackgroundType::Color => {
              writer.set_solid_background(Vec3::from(self.solid_background_color));
            }
            ViewerBackgroundType::Environment => {
              writer.set_hdr_env_background(self.default_env_background, 1.);
            }
          }
        }
      }
    });
  }
}

declare_entity!(SkyEnvironmentEntity);
declare_component!(SkyEnvironmentSunDirection, SkyEnvironmentEntity, Vec3<f32>);
declare_component!(SkyEnvironmentLuminance, SkyEnvironmentEntity, f32);
declare_component!(SkyEnvironmentTurbidity, SkyEnvironmentEntity, f32);
declare_component!(SkyEnvironmentRayleigh, SkyEnvironmentEntity, f32);
declare_component!(SkyEnvironmentMieCoefficient, SkyEnvironmentEntity, f32);
declare_component!(SkyEnvironmentDirectionalG, SkyEnvironmentEntity, f32);

pub fn register_sky_env_data_model() {
  global_database()
    .declare_entity::<SkyEnvironmentEntity>()
    .declare_component::<SkyEnvironmentSunDirection>()
    .declare_component::<SkyEnvironmentLuminance>()
    .declare_component::<SkyEnvironmentTurbidity>()
    .declare_component::<SkyEnvironmentRayleigh>()
    .declare_component::<SkyEnvironmentMieCoefficient>()
    .declare_component::<SkyEnvironmentDirectionalG>();
}
