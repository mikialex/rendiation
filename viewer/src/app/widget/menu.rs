use std::rc::Rc;

use interphaser::*;

#[derive(Clone)]
pub struct MenuModel {
  pub lists: Vec<MenuList>,
}

#[derive(Clone)]
pub struct MenuList {
  pub name: String,
  pub items: Vec<MenuItem>,
}

#[derive(Clone)]
pub enum MenuItem {
  SubList {
    name: String,
    list: MenuList,
    disabled: bool,
  },
  Item {
    name: String,
    disabled: bool,
    on_click: Rc<dyn Fn()>,
  },
  Separation,
}

impl IdentityKeyed for MenuList {
  type Key = String;

  fn key(&self) -> Self::Key {
    self.name.clone()
  }
}

pub fn menu() -> impl UIComponent<MenuModel> {
  Container::size((100., 100.)).wrap(
    For::by(|_| Child::flex(menu_title(), 1.)) //
      .extend(Flex::column())
      .lens(lens!(MenuModel, lists)),
  )
}

fn menu_title() -> impl UIComponent<MenuList> {
  Container::size((100., 100.)).wrap(
    Text::default()
      .bind(|s, t| s.content.set(t)) //
      .lens(lens!(MenuList, name)),
  )
}
