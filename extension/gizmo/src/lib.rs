// // mod gizmo;
// // pub use gizmo::*;
// // mod interactive;
// // pub use interactive::*;
// // mod view;
// // pub use view::*;

// use reactive::AllocIdx;
// use rendiation_scene_core::*;

// pub struct EventCtx3D;

// pub trait View<T> {
//   fn event(&mut self, model: &mut T, event: &EventCtx3D);
// }

// struct WidgetModel<T> {
//   model: AllocIdx<SceneModelEntity>,
//   nodes: AllocIdx<SceneNodeEntity>,
//   material: AllocIdx<FlatMaterialEntity>,
//   mesh: AllocIdx<SceneModelEntity>,
//   on_click: Option<Box<dyn FnMut(&mut T) + Send + Sync>>,
//   on_mouse_hover: Option<Box<dyn FnMut(&mut T) + Send + Sync>>,
//   on_mouse_down: Option<Box<dyn FnMut(&mut T) + Send + Sync>>,
// }

// impl<T> View<T> for WidgetModel<T> {
//   fn event(&mut self, model: &mut T, event: &EventCtx3D) {
//     todo!()
//   }
// }

// impl<T> WidgetModel<T> {
//   pub fn color(&mut self, color: usize) -> &mut Self {
//     self
//   }

//   pub fn matrix(&mut self, color: usize) -> &mut Self {
//     self
//   }
// }

// pub trait ViewController<T> {
//   /// run when app init.
//   fn init_view(&self) -> T;

//   /// run every frame
//   fn update_view(&self, view: &mut T);
// }

// struct Button<T> {
//   color: usize,
//   name: usize,
//   other_fancy_property: usize,
//   cb: Option<Box<dyn FnMut(&mut T) + Send + Sync>>,
// }

// impl<T> View<T> for Button<T> {
//   fn event(&mut self, model: &mut T, event: &EventCtx3D) {
//     todo!()
//   }
// }

// struct MyApplication {
//   a: usize,
//   b: usize,
// }

// impl ViewController<Button<Self>> for MyApplication {
//   fn init_view(&self) -> Button<Self> {
//     // Button::default()
//     todo!()
//   }

//   fn update_view(&self, view: &mut Button<Self>) {
//     view.name = self.a
//   }
// }
