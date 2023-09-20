use crate::*;

// todo, use container directly?
#[derive(Default)]
pub struct AbsoluteAnchor {
  position: UIPosition,
}

trivial_stream_impl!(AbsoluteAnchor);
impl<C: View> ViewNester<C> for AbsoluteAnchor
where
  for<'a> &'a mut C: IntoIterator<Item = &'a mut AbsChild>,
{
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C) {
    match detail {
      ViewRequest::Layout(p) => {
        match p {
          LayoutProtocol::DoLayout {
            constraint,
            output,
            ctx,
          } => {
            // we just pass the parent constraint to children, so the anchor itself is
            // transparent to children
            inner.into_iter().for_each(|child| {
              child.inner.layout(*constraint, ctx);
            });

            **output = constraint.max().with_default_baseline();
          }
          LayoutProtocol::PositionAt(position) => {
            self.position = *position;
            inner.into_iter().for_each(|child| {
              child.set_position(child.position);
            });
          }
        }
      }
      ViewRequest::Encode(builder) => {
        builder.push_translate(self.position);
        inner.draw(builder);
        builder.pop_translate()
      }
      _ => inner.request(detail),
    }
  }
}

pub fn absolute_group() -> ComponentArray<AbsChild> {
  Vec::new().into()
}

pub struct AbsChild {
  pub position: UIPosition,
  pub inner: Box<dyn View>,
}

impl AbsChild {
  pub fn new(inner: impl View + 'static) -> Self {
    Self {
      inner: Box::new(inner),
      position: Default::default(),
    }
  }

  #[must_use]
  pub fn with_position(mut self, position: impl Into<UIPosition>) -> Self {
    self.position = position.into();
    self
  }
}

impl Stream for AbsChild {
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    self.inner.poll_next_unpin(cx)
  }
}

impl View for AbsChild {
  fn request(&mut self, detail: &mut ViewRequest) {
    self.inner.request(detail)
  }
}
