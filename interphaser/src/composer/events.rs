// use winit::event::*;

// use crate::*;

// #[derive(Default)]
// pub struct EventHandler<X: EventHandlerType> {
//   state: X,
//   pub events: EventSource<X::Event>,
// }

// impl<C, X: EventHandlerType> ReactiveUpdateNester<C> for EventHandler<X> {
//   fn poll_update_inner(self: Pin<&mut Self>, _: &mut Context<'_>, _: &mut C) -> Poll<Option<()>>
// {     Poll::Pending
//   }
// }

// pub trait EventHandlerLike<C> {
//   fn handle_event(&mut self, event: &mut EventCtx, inner: &mut C);
//   fn should_handle_in_bubble(&self) -> bool;
// }

// pub struct EventHandlerGroup<C> {
//   before_handlers: Vec<Box<dyn EventHandlerLike<C>>>,
//   after_handlers: Vec<Box<dyn EventHandlerLike<C>>>,
// }

// impl<C> Default for EventHandlerGroup<C> {
//   fn default() -> Self {
//     Self {
//       before_handlers: Default::default(),
//       after_handlers: Default::default(),
//     }
//   }
// }

// impl<C> EventHandlerGroup<C> {
//   #[must_use]
//   pub fn with(mut self, handler: impl EventHandlerLike<C> + 'static) -> Self {
//     if handler.should_handle_in_bubble() {
//       self.after_handlers.push(Box::new(handler));
//     } else {
//       self.before_handlers.push(Box::new(handler));
//     }
//     self
//   }
// }

// impl<C: Eventable> EventableNester<C> for EventHandlerGroup<C> {
//   fn event(&mut self, event: &mut EventCtx, inner: &mut C) {
//     self
//       .before_handlers
//       .iter_mut()
//       .for_each(|handler| handler.handle_event(event, inner));

//     inner.event(event);

//     self
//       .after_handlers
//       .iter_mut()
//       .for_each(|handler| handler.handle_event(event, inner));
//   }
// }

// impl<X, C: Presentable> PresentableNester<C> for EventHandlerGroup<X> {
//   fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
//     inner.render(builder);
//   }
// }

// impl<X, C: LayoutAble> LayoutAbleNester<C> for EventHandlerGroup<X> {
//   fn layout(
//     &mut self,
//     constraint: LayoutConstraint,
//     ctx: &mut LayoutCtx,
//     inner: &mut C,
//   ) -> LayoutResult {
//     inner.layout(constraint, ctx)
//   }

//   fn set_position(&mut self, position: UIPosition, inner: &mut C) {
//     inner.set_position(position)
//   }
// }

// impl<X, C: HotAreaProvider> HotAreaNester<C> for EventHandlerGroup<X> {
//   fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
//     inner.is_point_in(point)
//   }
// }

// impl<C, X: EventHandlerImpl<C>> EventHandlerLike<C> for EventHandler<X> {
//   fn handle_event(&mut self, event: &mut EventCtx, inner: &mut C) {
//     if let Some(e) = self.state.downcast_event(event, inner) {
//       self.events.emit(e)
//     }
//   }
//   fn should_handle_in_bubble(&self) -> bool {
//     self.state.should_handle_in_bubble()
//   }
// }

// impl<X: EventHandlerImpl<C>, C: Eventable> EventableNester<C> for EventHandler<X> {
//   fn event(&mut self, event: &mut EventCtx, inner: &mut C) {
//     if !self.state.should_handle_in_bubble() {
//       self.handle_event(event, inner);
//     }

//     inner.event(event);

//     if self.state.should_handle_in_bubble() {
//       self.handle_event(event, inner);
//     }
//   }
// }

// impl<X: EventHandlerType, C: Presentable> PresentableNester<C> for EventHandler<X> {
//   fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
//     inner.render(builder);
//   }
// }

// impl<X: EventHandlerType, C: LayoutAble> LayoutAbleNester<C> for EventHandler<X> {
//   fn layout(
//     &mut self,
//     constraint: LayoutConstraint,
//     ctx: &mut LayoutCtx,
//     inner: &mut C,
//   ) -> LayoutResult {
//     inner.layout(constraint, ctx)
//   }

//   fn set_position(&mut self, position: UIPosition, inner: &mut C) {
//     inner.set_position(position)
//   }
// }

// impl<X: EventHandlerType, C: HotAreaProvider> HotAreaNester<C> for EventHandler<X> {
//   fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
//     inner.is_point_in(point)
//   }
// }

// pub trait EventHandlerType: Default {
//   type Event: 'static;
// }

// pub trait EventHandlerImpl<C>: EventHandlerType {
//   fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a
// Self::Event>;   fn should_handle_in_bubble(&self) -> bool {
//     false
//   }
// }

// #[derive(Default)]
// pub struct MouseDown;
// pub type MouseDownHandler = EventHandler<MouseDown>;
// impl EventHandlerType for MouseDown {
//   type Event = ();
// }
// impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseDown {
//   fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a Self::Event>
// {     if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
//       if inner.is_point_in(event.states.mouse_position) {
//         return Some(&());
//       }
//     }
//     None
//   }
// }

// #[derive(Default)]
// pub struct MouseUp;
// pub type MouseUpHandler = EventHandler<MouseUp>;
// impl EventHandlerType for MouseUp {
//   type Event = ();
// }
// impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseUp {
//   fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a Self::Event>
// {     if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.event) {
//       if inner.is_point_in(event.states.mouse_position) {
//         return Some(&());
//       }
//     }
//     None
//   }
// }

// #[derive(Default)]
// pub struct Click {
//   mouse_down: bool,
// }

// pub type ClickHandler = EventHandler<Click>;
// impl EventHandlerType for Click {
//   type Event = ();
// }
// impl<C: HotAreaProvider> EventHandlerImpl<C> for Click {
//   fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a Self::Event>
// {     if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
//       if inner.is_point_in(event.states.mouse_position) {
//         self.mouse_down = true;
//       }
//     } else if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.event) {
//       if self.mouse_down && inner.is_point_in(event.states.mouse_position) {
//         return Some(&());
//       }
//       self.mouse_down = false;
//     }
//     None
//   }
// }

// #[derive(Default)]
// pub struct MouseMove;
// pub type MouseMoveHandler = EventHandler<MouseMove>;
// impl EventHandlerType for MouseMove {
//   type Event = ();
// }
// impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseMove {
//   fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a Self::Event>
// {     if let Some(position) = mouse_move(event.event) {
//       if inner.is_point_in((position.x as f32, position.y as f32).into()) {
//         return Some(&());
//       }
//     }
//     None
//   }
// }

// #[derive(Default)]
// pub struct MouseIn {
//   is_mouse_in: bool,
// }

// pub type MouseInHandler = EventHandler<MouseIn>;
// impl EventHandlerType for MouseIn {
//   type Event = ();
// }
// impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseIn {
//   fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a Self::Event>
// {     if let Some(position) = mouse_move(event.event) {
//       if inner.is_point_in((position.x as f32, position.y as f32).into()) {
//         if !self.is_mouse_in {
//           self.is_mouse_in = true;
//           return Some(&());
//         }
//         self.is_mouse_in = true;
//       } else {
//         self.is_mouse_in = false;
//       }
//     }
//     None
//   }
// }

// #[derive(Default)]
// pub struct MouseOut {
//   is_mouse_in: bool,
// }
// pub type MouseOutHandler = EventHandler<MouseOut>;
// impl EventHandlerType for MouseOut {
//   type Event = ();
// }
// impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseOut {
//   fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a Self::Event>
// {     if let Some(position) = mouse_move(event.event) {
//       if !inner.is_point_in((position.x as f32, position.y as f32).into()) {
//         if self.is_mouse_in {
//           self.is_mouse_in = false;
//           return Some(&());
//         }
//         self.is_mouse_in = false;
//       } else {
//         self.is_mouse_in = true;
//       }
//     }
//     None
//   }
// }

// // these downcast utils below is useful for downstream crates because they shouldn't care about
// impl // details so they are public and export, we should consider warp them in a namespace in the
// future // to prevent potential name collisions

// pub fn window_event<'a>(event: &'a Event<()>) -> Option<&'a WindowEvent<'a>> {
//   match event {
//     Event::WindowEvent { event, .. } => Some(event),
//     _ => None,
//   }
// }

// pub fn mouse(event: &Event<()>) -> Option<(MouseButton, ElementState)> {
//   window_event(event).and_then(|e| match e {
//     WindowEvent::MouseInput { state, button, .. } => Some((*button, *state)),
//     _ => None,
//   })
// }

// pub fn mouse_move(event: &Event<()>) -> Option<winit::dpi::PhysicalPosition<f64>> {
//   window_event(event).and_then(|e| match e {
//     WindowEvent::CursorMoved { position, .. } => Some(*position),
//     _ => None,
//   })
// }
