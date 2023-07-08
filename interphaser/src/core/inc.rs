use std::task::Context;

// use incremental::*;

// use crate::*;

/// View type could generic over any state T, as long as the T could provide
/// given logic for view type
pub trait UIView {
  type Event;
  type React;

  fn event(&mut self, request: ViewRequest<Self::Event>, cb: impl FnMut(ViewReact<Self::React>));
}

pub enum ViewReact<V> {
  ViewEvent(V),
  LayoutChanged,
  RenderChanged,
}

pub enum PlatformInput {}

pub enum PlatformRequest<'a> {
  Event {
    event: &'a PlatformInput,
  },
  Layout {
    parent_constraint: usize,
    cb: &'a mut usize,
  },
  Rendering {
    ctx: &'a usize,
  },
}

pub enum ViewRequest<'a, T> {
  Platform(&'a PlatformRequest<'a>),
  State(&'a T, &'a Context<'a>),
}

pub enum TextBoxEvent {
  Submit(String),
  Hovering,
  Select,
}

pub struct TextBox {
  texting: String,
  placeholder: String,
}

// struct UISystem;

// async fn ui(cx: &UISystem) {
//   let layout_frame = main_frame(cx);
//   let tool_bar = toolbar_view(cx);

//   let view_3d = load_main_3d_view(cx);

//   loop {
//     match tool_bar.next().await {
//       LoadFile => {
//         let file = file_select_view(cx).await;
//         view_3d.load_file(file).await;
//       }
//       Exit => {
//         return;
//       }
//     }
//   }
// }

// pub struct ReactiveTextureBox {
//   texting: Box<dyn Stream<Item = String>>,
//   placeholder: Box<dyn Stream<Item = String>>,
// }

pub enum TextBoxDelta {
  Text(String),
  Placeholder(String),
}

impl UIView for TextBox {
  type Event = TextBoxDelta;
  type React = TextBoxEvent;

  fn event(
    &mut self,
    request: ViewRequest<Self::Event>,
    mut cb: impl FnMut(ViewReact<Self::React>),
  ) {
    match request {
      ViewRequest::Platform(event) => {
        match event {
          PlatformRequest::Event { event: _ } => {
            let react = false;
            // omit
            // processing platform events
            // modify self editing text, and dispatch events

            if react {
              cb(ViewReact::ViewEvent(TextBoxEvent::Submit(
                self.texting.clone(),
              )))
            }
          }
          PlatformRequest::Layout { .. } => cb(ViewReact::RenderChanged),
          PlatformRequest::Rendering { .. } => todo!(),
        }
      }
      ViewRequest::State(delta, _) => {
        match delta {
          TextBoxDelta::Text(t) => self.texting = t.clone(),
          TextBoxDelta::Placeholder(t) => self.placeholder = t.clone(),
        }
        cb(ViewReact::LayoutChanged)
      }
    }
  }
}

// struct ViewDeltaTransform<T, V: View, F> {
//   view: V,
//   transform: F,
//   phantom: std::marker::PhantomData<T>,
// }

// impl<T, V: View, F> View for ViewDeltaTransform<T, V, F>
// where
//   F: Fn(&T) -> V::Event,
//   V::Event: 'static,
// {
//   type Event = T;
//   type React = V::React;

//   fn event(&mut self, request: ViewRequest<Self::Event>, cb: impl FnMut(ViewReact<Self::React>))
// {     match request {
//       ViewRequest::Platform(event) => {
//         let mapped = ViewRequest::Platform(event);
//         self.view.event(mapped, cb)
//       }
//       ViewRequest::State(delta, cx) => {
//         let delta = (self.transform)(&delta);
//         let mapped = ViewRequest::State(&delta, cx);
//         self.view.event(mapped, cb)
//       }
//     };
//   }
// }
