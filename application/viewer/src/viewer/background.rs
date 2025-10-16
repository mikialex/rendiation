use crate::*;

#[derive(Debug, PartialEq, Clone, Copy)]
enum ViewerBackgroundType {
  Color,
  Gradient,
  Environment,
  //   Sky,
}

pub struct ViewerBackgroundState {
  current: ViewerBackgroundType,
  default_env_background: EntityHandle<SceneTextureCubeEntity>,
  solid_background_color: [f32; 3],
  gradient: SceneGradientBackgroundParam,
}

impl ViewerBackgroundState {
  pub fn setup_background(&self, writer: &mut SceneWriter) {
    match self.current {
      ViewerBackgroundType::Color => {
        writer.set_solid_background(Vec3::from(self.solid_background_color));
      }
      ViewerBackgroundType::Environment => {
        writer.set_hdr_env_background(self.default_env_background, 1., Mat4::identity());
      }
      ViewerBackgroundType::Gradient => {
        writer.set_gradient_background(self.gradient.clone());
      }
    }
  }
}

const DEFAULT_BACKGROUND: Vec3<f32> = Vec3::new(0.8, 0.8, 0.8);

const SKY_BLUE: Vec3<f32> = Vec3::new(0.373, 0.753, 0.922);
const GROUND_GREEN: Vec3<f32> = Vec3::new(0.667, 0.761, 0.608);

impl ViewerBackgroundState {
  pub fn init(writer: &mut SceneWriter) -> Self {
    let default_env_background = load_example_cube_tex(writer);
    let s = Self {
      current: ViewerBackgroundType::Color,
      default_env_background,
      solid_background_color: DEFAULT_BACKGROUND.into(),
      gradient: SceneGradientBackgroundParam {
        transform: Mat4::identity(),
        color_and_stops: vec![
          SKY_BLUE.expand_with(0.0),
          SKY_BLUE.expand_with(0.3),
          Vec4::new(1.0, 1.0, 1.0, 0.5),
          GROUND_GREEN.expand_with(0.5),
          GROUND_GREEN.expand_with(1.0),
        ],
      },
    };
    s.setup_background(writer);
    s
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
            ViewerBackgroundType::Gradient,
            "Gradient",
          );
          ui.selectable_value(
            &mut self.current,
            ViewerBackgroundType::Environment,
            "EnvironmentMap",
          );
        });

      {
        let mut writer = SceneWriter::from_global(scene);
        match self.current {
          ViewerBackgroundType::Color => {
            ui.color_edit_button_rgb(&mut self.solid_background_color);
            writer.set_solid_background(Vec3::from(self.solid_background_color))
          }
          ViewerBackgroundType::Environment => {} // not editable for now
          ViewerBackgroundType::Gradient => {}    // not editable for now
        }
        if self.current != previous {
          self.setup_background(&mut writer);
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
