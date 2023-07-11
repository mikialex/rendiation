use crate::*;

#[derive(Default)]
pub struct AbsoluteAnchor {
  position: UIPosition,
}

trivial_stream_nester_impl!(AbsoluteAnchor);
impl<C: View> ViewNester<C> for AbsoluteAnchor
where
  for<'a> &'a mut C: IntoIterator<Item = &'a mut AbsChild>,
{
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C) {
    match detail {
      ViewRequest::Event(_) => inner.request(detail),
      ViewRequest::Layout(p) => {
        match p {
          LayoutProtocol::DoLayout {
            constraint,
            output,
            ctx,
          } => {
            // we just pass the parent constraint to children, so the anchor itself is
            // transparent to children
            let mut result = Default::default();
            inner.into_iter().for_each(|child| {
              child
                .inner
                .request(&mut ViewRequest::Layout(LayoutProtocol::DoLayout {
                  constraint: *constraint,
                  output: &mut result,
                  ctx,
                }));
            });

            **output = constraint.max().with_default_baseline();
          }
          LayoutProtocol::PositionAt(position) => {
            self.position = *position;
            inner.into_iter().for_each(|child| {
              child.request(&mut ViewRequest::Layout(LayoutProtocol::PositionAt(
                child.position,
              )));
            });
          }
        }
      }
      ViewRequest::Encode(builder) => {
        builder.push_offset(self.position);
        inner.request(&mut ViewRequest::Encode(builder));
        builder.pop_offset()
      }
    }
  }
}

impl<C: HotAreaProvider> HotAreaNester<C> for AbsoluteAnchor {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    inner.is_point_in(point)
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
