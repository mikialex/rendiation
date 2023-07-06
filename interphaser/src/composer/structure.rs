use crate::*;

// pub struct If<C> {
//   should_render: Box<dyn Fn(&T) -> bool>,
//   func: Box<dyn Fn(&T) -> C>,
//   inner: Option<C>,
// }

// impl<C> If<C>
// where
//   C: Component,
// {
//   pub fn condition<F, SF>(should_render: SF, func: F) -> Self
//   where
//     SF: Fn(&T) -> bool + 'static,
//     F: Fn(&T) -> C + 'static,
//   {
//     Self {
//       should_render: Box::new(should_render),
//       func: Box::new(func),
//       inner: None,
//     }
//   }

//   pub fn else_condition<F, EC>(self, func: F) -> Else<C, EC>
//   where
//     F: Fn(&T) -> EC + 'static,
//   {
//     Else {
//       if_com: self,
//       func: Box::new(func),
//       inner: None,
//     }
//   }
// }

// impl<C> Component for If<C>
// where
//   C: Component,
// {
//   fn update(&mut self, model: &ctx: &mut UpdateCtx) {
//     if (self.should_render)(model) {
//       if let Some(inner) = &mut self.inner {
//         inner.update(ctx);
//       } else {
//         self.inner = Some((self.func)(model));
//       }
//     } else {
//       self.inner = None;
//     }
//   }

//   fn event(&mut self, event: &mut crate::EventCtx) {
//     if let Some(inner) = &mut self.inner {
//       inner.event(event)
//     }
//   }
// }

// impl<C: LayoutAble> LayoutAble for If<C> {
//   fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutResult {
//     if let Some(inner) = &mut self.inner {
//       inner.layout(constraint, ctx)
//     } else {
//       LayoutResult {
//         size: constraint.min(),
//         baseline_offset: 0.,
//       }
//     }
//   }

//   fn set_position(&mut self, position: UIPosition) {
//     if let Some(inner) = &mut self.inner {
//       inner.set_position(position)
//     }
//   }
// }

// impl<C: Presentable> Presentable for If<C> {
//   fn render(&mut self, builder: &mut PresentationBuilder) {
//     if let Some(inner) = &mut self.inner {
//       inner.render(builder)
//     }
//   }
// }

// pub struct Else<C, EC> {
//   if_com: If<C>,
//   func: Box<dyn Fn(&T) -> EC>,
//   inner: Option<EC>,
// }

// impl<C, EC> Component for Else<C, EC>
// where
//   C: Component,
//   EC: Component,
// {
//   fn update(&mut self, model: &ctx: &mut UpdateCtx) {
//     self.if_com.update(ctx);

//     if self.if_com.inner.is_none() {
//       if let Some(inner) = &mut self.inner {
//         inner.update(ctx);
//       } else {
//         self.inner = Some((self.func)(model));
//       }
//     } else {
//       self.inner = None
//     }
//   }

//   fn event(&mut self, event: &mut crate::EventCtx) {
//     if let Some(inner) = &mut self.inner {
//       inner.event(event)
//     } else {
//       self.if_com.event(event);
//     }
//   }
// }

// impl<C: LayoutAble, EC: LayoutAble> LayoutAble for Else<C, EC> {
//   fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutResult {
//     if let Some(inner) = &mut self.inner {
//       inner.layout(constraint, ctx)
//     } else {
//       self.if_com.layout(constraint, ctx)
//     }
//   }

//   fn set_position(&mut self, position: UIPosition) {
//     if let Some(inner) = &mut self.inner {
//       inner.set_position(position)
//     } else {
//       self.if_com.set_position(position);
//     }
//   }
// }

// impl<C: Presentable, EC: Presentable> Presentable for Else<C, EC> {
//   fn render(&mut self, builder: &mut PresentationBuilder) {
//     if let Some(inner) = &mut self.inner {
//       inner.render(builder)
//     } else {
//       self.if_com.render(builder);
//     }
//   }
// }

// /// if item's key not changed, we consider this item should update not destroy
// pub trait IdentityKeyed {
//   type Key: PartialEq;
//   fn key(&self) -> Self::Key;
// }

// pub struct For<T: IdentityKeyed, C> {
//   children: Vec<(T::Key, C)>,
//   mapper: Box<dyn Fn(usize) -> C>,
// }

// impl<C> For<C>
// where
//   T: IdentityKeyed,
//   C: Component,
// {
//   pub fn by<F>(mapper: F) -> Self
//   where
//     F: Fn(usize) -> C + 'static,
//   {
//     Self {
//       children: Vec::new(),
//       mapper: Box::new(mapper),
//     }
//   }
// }

// impl<C> Component<Vec> for For<C>
// where
//   T: 'static + IdentityKeyed + Clone,
//   C: Component,
// {
//   fn update(&mut self, model: &Vec, ctx: &mut UpdateCtx) {
//     // todo should optimize
//     self.children = model
//       .iter()
//       .enumerate()
//       .map(|(index, item)| {
//         let new_key = item.key();

//         if let Some(previous) = self.children.iter().position(|cached| cached.0 == new_key) {
//           // move
//           self.children.swap_remove(previous)
//         } else {
//           // new
//           (new_key, (self.mapper)(index))
//         }
//       })
//       .collect();
//     // and not exist will be drop

//     self
//       .children
//       .iter_mut()
//       .zip(model)
//       .for_each(|((_, c), m)| c.update(m, ctx))
//   }

//   fn event(&mut self, model: &mut Vec, event: &mut crate::EventCtx) {
//     self
//       .children
//       .iter_mut()
//       .zip(model)
//       .for_each(|((_, item), model)| item.event(event))
//   }
// }

// type IterType<'a, C: 'static, T: 'static + IdentityKeyed> =
//   impl Iterator<Item = &'a mut C> + 'a + ExactSizeIterator;

// impl<'a, T: 'static + IdentityKeyed, C: 'static> IntoIterator for &'a mut For<C> {
//   type Item = &'a mut C;
//   type IntoIter = IterType<'a, C, T>;

//   fn into_iter(self) -> IterType<'a, C, T> {
//     self.children.iter_mut().map(|(_, c)| c)
//   }
// }

// impl<T: IdentityKeyed, C: Presentable> Presentable for For<C> {
//   fn render(&mut self, builder: &mut PresentationBuilder) {
//     self
//       .children
//       .iter_mut()
//       .for_each(|(_, c)| c.render(builder))
//   }
// }

#[derive(Default)]
pub struct ComponentArray<C> {
  pub children: Vec<C>,
}

impl<C> From<Vec<C>> for ComponentArray<C> {
  fn from(children: Vec<C>) -> Self {
    Self { children }
  }
}

impl<X> ComponentArray<X> {
  #[must_use]
  pub fn child(mut self, x: X) -> Self {
    self.children.push(x);
    self
  }
}

type IterType2<'a, C: 'static> = impl Iterator<Item = &'a mut C> + 'a + ExactSizeIterator;

impl<'a, C: 'static> IntoIterator for &'a mut ComponentArray<C> {
  type Item = &'a mut C;
  type IntoIter = IterType2<'a, C>;

  fn into_iter(self) -> IterType2<'a, C> {
    self.children.iter_mut()
  }
}

impl<C: Presentable> Presentable for ComponentArray<C> {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.children.iter_mut().for_each(|c| c.render(builder))
  }
}

impl<C> Eventable for ComponentArray<C>
where
  C: Eventable,
{
  fn event(&mut self, event: &mut crate::EventCtx) {
    self.children.iter_mut().for_each(|c| c.event(event))
  }
}

// /// using Enum Discriminant to decide if we should cache UI Component instance
// pub struct EnumMatcher {
//   com: Option<(Box<dyn Component>, std::mem::Discriminant)>,
//   matcher: Box<dyn Fn(&T) -> Box<dyn Component>>,
// }

// impl EnumMatcher {
//   pub fn by(matcher: impl Fn(&T) -> Box<dyn Component> + 'static) -> Self {
//     Self {
//       com: None,
//       matcher: Box::new(matcher),
//     }
//   }
// }

// impl Component for EnumMatcher {
//   fn event(&mut self, event: &mut EventCtx) {
//     if let Some((com, _)) = &mut self.com {
//       com.event(event)
//     }
//   }

//   fn update(&mut self, model: &ctx: &mut UpdateCtx) {
//     let current_dis = std::mem::discriminant(model);

//     let com = if let Some((com, dis)) = &mut self.com {
//       if current_dis != *dis {
//         *com = (self.matcher)(model);
//         *dis = current_dis;
//       };
//       com
//     } else {
//       &mut self.com.insert(((self.matcher)(model), current_dis)).0
//     };

//     com.update(ctx)
//   }
// }

// impl LayoutAble for EnumMatcher {
//   fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutResult {
//     if let Some((com, _)) = &mut self.com {
//       com.layout(constraint, ctx)
//     } else {
//       LayoutResult {
//         size: constraint.min(),
//         baseline_offset: 0.,
//       }
//     }
//   }

//   fn set_position(&mut self, position: UIPosition) {
//     if let Some((inner, _)) = &mut self.com {
//       inner.set_position(position)
//     }
//   }
// }

// impl Presentable for EnumMatcher {
//   fn render(&mut self, builder: &mut PresentationBuilder) {
//     if let Some((inner, _)) = &mut self.com {
//       inner.render(builder)
//     }
//   }
// }
