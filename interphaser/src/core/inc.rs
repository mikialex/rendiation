use std::task::Context;

// use incremental::*;

// use crate::*;

/// View type could generic over any state T, as long as the T could provide
/// given logic for view type
trait View {
  type Event;
  type React;

  fn event(
    &mut self,
    request: ViewRequest<Self::Event>,
    cb: &mut dyn FnMut(ViewReaction<Self::React>),
  );
}

pub enum ViewReaction<V> {
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

pub enum TextBoxDelta {
  Text(String),
  Placeholder(String),
}

impl View for TextBox {
  type Event = TextBoxDelta;
  type React = TextBoxEvent;

  fn event(
    &mut self,
    request: ViewRequest<Self::Event>,
    cb: &mut dyn FnMut(ViewReaction<Self::React>),
  ) {
    match request {
      ViewRequest::Platform(_event) => {
        let react = false;
        // omit
        // processing platform events
        // modify self editing text, and dispatch events

        if react {
          cb(ViewReaction::ViewEvent(TextBoxEvent::Submit(
            self.texting.clone(),
          )))
        }
      }
      ViewRequest::State(delta, _) => {
        match delta {
          TextBoxDelta::Text(t) => self.texting = t.clone(),
          TextBoxDelta::Placeholder(t) => self.placeholder = t.clone(),
        }
        cb(ViewReaction::LayoutChanged)
      }
    }
  }
}

// struct ViewDeltaTransform<T: Incremental, V: View> {
//   view: V,
//   binding: Box<dyn Fn(&DeltaOf<T>, &mut V, &mut dyn FnMut(ViewReaction<V::Event>))>,
// }

// impl<T: Incremental, V: View> View for ViewDeltaTransform<T, V> {
//   type Input = T::Delta;
//   type Event = V::Event;

//   fn event(
//     &mut self,
//     request: ViewRequest<Self::Input>,
//     cb: &mut dyn FnMut(ViewReaction<Self::Event>),
//   ) {
//     match request {
//       ViewRequest::Platform(event) => {
//         self.view.event(request, cb)
//       }
//       ViewRequest::State(delta, _) => {
//         match delta {
//           TextBoxDelta::Text(t) => self.texting = t.clone(),
//           TextBoxDelta::Placeholder(t) => self.placeholder = t.clone(),
//         }
//         cb(ViewReaction::LayoutChanged)
//       }
//     }
//   }
// }
