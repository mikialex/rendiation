//! Declare the bindings once, and use the same declaration for both shader side and pass side.
//!
//! Traditionally the `bind_by` call order in shader building must be exactly same as the `bind`
//! call order in pass setup, which is an implicit contract and easy to break. By implementing
//! [BindingDeclare], the binding order is guaranteed by construction because both side execute
//! the same declare function, and the shader side still gets the fully typed shader instances.
//!
//! The [BindKind] decides what each bind item produces, the [Binder] does the actual binding.
//! They are separated because the binder holds a mutable borrow of the builder, if the result type
//! depends on the binder, the result will keep borrowing the builder.

use crate::*;

pub trait BindItem: AbstractShaderBindingSource + AbstractBindingSource {}
impl<T: AbstractShaderBindingSource + AbstractBindingSource> BindItem for T {}

pub trait BindKind {
  type Out<T: BindItem>;
}

/// bind in current shader stage, produce the shader instance
pub struct ShaderKind;
impl BindKind for ShaderKind {
  type Out<T: BindItem> = T::ShaderBindResult;
}

/// bind in both vertex(or mesh) and fragment stage, produce the accessor of both stage instance
pub struct GraphicsPairKind;
impl BindKind for GraphicsPairKind {
  type Out<T: BindItem> = GraphicsPairInputNodeAccessor<T>;
}

/// bind in pass, produce nothing
pub struct PassKind;
impl BindKind for PassKind {
  type Out<T: BindItem> = ();
}

pub trait Binder {
  type Kind: BindKind;
  fn bind<T: BindItem>(&mut self, item: &T) -> <Self::Kind as BindKind>::Out<T>;
  /// bind the items in the scope to the given bindgroup index
  fn with_group<R>(&mut self, group: usize, f: impl FnOnce(&mut Self) -> R) -> R;
}

pub trait BindingDeclare {
  /// the typed bindings, usually a struct that each field is `K::Out<ItemType>`
  type Bindings<K: BindKind>;
  fn declare<B: Binder>(&self, b: &mut B) -> Self::Bindings<B::Kind>;
}

/// bind in current shader stage, must be used inside a shader stage
pub struct ShaderBinder<'a>(pub &'a mut ShaderBindGroupBuilder);
impl Binder for ShaderBinder<'_> {
  type Kind = ShaderKind;
  fn bind<T: BindItem>(&mut self, item: &T) -> T::ShaderBindResult {
    self.0.bind_by(item)
  }
  fn with_group<R>(&mut self, group: usize, f: impl FnOnce(&mut Self) -> R) -> R {
    let before = self.0.set_binding_slot(group);
    let r = f(self);
    self.0.set_binding_slot(before);
    r
  }
}

/// bind in both vertex(or mesh) and fragment stage, must be used outside any shader stage.
///
/// note, the writeable storage buffer is not allowed in vertex stage, so such item should not be
/// bound by this binder.
pub struct GraphicsPairBinder<'a>(pub &'a mut ShaderRenderPipelineBuilder);
impl Binder for GraphicsPairBinder<'_> {
  type Kind = GraphicsPairKind;
  fn bind<T: BindItem>(&mut self, item: &T) -> GraphicsPairInputNodeAccessor<T> {
    BindingPreparer::new(item).using_graphics_pair(self.0, |_, _| {})
  }
  fn with_group<R>(&mut self, group: usize, f: impl FnOnce(&mut Self) -> R) -> R {
    let before = self.0.set_binding_slot(group);
    let r = f(self);
    self.0.set_binding_slot(before);
    r
  }
}

pub struct PassBinder<'a>(pub &'a mut BindingBuilder);
impl Binder for PassBinder<'_> {
  type Kind = PassKind;
  fn bind<T: BindItem>(&mut self, item: &T) {
    self.0.bind(item);
  }
  fn with_group<R>(&mut self, group: usize, f: impl FnOnce(&mut Self) -> R) -> R {
    let before = self.0.set_binding_slot(group);
    let r = f(self);
    self.0.set_binding_slot(before);
    r
  }
}
