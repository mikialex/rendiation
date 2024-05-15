use crate::*;

pub struct SelectionProvider<T> {
  inner: T,
  // pub picker: MeshBufferIntersectConfig,
}

impl<T> StatefulView for SelectionProvider<T> {
  fn update_state(&mut self, cx: &mut StateCx) {
    todo!()
  }

  fn update_view(&mut self, cx: &mut StateCx) {
    todo!()
  }

  fn clean_up(&mut self, cx: &mut StateCx) {
    todo!()
  }
}

pub struct ViewerSelection {
  model: Option<AllocIdx<SceneModelEntity>>,
}
