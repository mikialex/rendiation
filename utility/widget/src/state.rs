use crate::*;

pub struct StateCtxInject<T, V> {
  pub view: V,
  pub state: T,
}

impl<T: 'static, V: Widget> Widget for StateCtxInject<T, V> {
  fn update_view(&mut self, cx: &mut DynCx) {
    cx.scoped_cx(&mut self.state, |cx| {
      self.view.update_view(cx);
    })
  }

  fn update_state(&mut self, cx: &mut DynCx) {
    cx.scoped_cx(&mut self.state, |cx| {
      self.view.update_state(cx);
    })
  }
  fn clean_up(&mut self, cx: &mut DynCx) {
    self.view.clean_up(cx)
  }
}

pub struct StateCtxPick<V, F, T1, T2> {
  pub view: V,
  pub pick: F,
  pub phantom: PhantomData<(T1, T2)>,
}

impl<T1: 'static, T2: 'static, F: Fn(&mut T1) -> &mut T2, V: Widget> Widget
  for StateCtxPick<V, F, T1, T2>
{
  fn update_view(&mut self, cx: &mut DynCx) {
    unsafe {
      let s = cx.get_cx_ptr::<T1>().unwrap();
      let picked = (self.pick)(&mut *s);

      cx.scoped_cx(picked, |cx| {
        self.view.update_view(cx);
      });
    }
  }

  fn update_state(&mut self, cx: &mut DynCx) {
    unsafe {
      let s = cx.get_cx_ptr::<T1>().unwrap();
      let picked = (self.pick)(&mut *s);

      cx.scoped_cx(picked, |cx| {
        self.view.update_state(cx);
      });
    }
  }
  fn clean_up(&mut self, cx: &mut DynCx) {
    self.view.clean_up(cx)
  }
}

#[test]
fn test_state_cx() {
  let mut cx = DynCx::default();

  let mut a: usize = 1;
  let mut b: usize = 2;

  unsafe {
    cx.register_cx(&mut a);
    assert_eq!(*cx.get_cx_ref::<usize>(), 1);

    cx.register_cx(&mut b);
    assert_eq!(*cx.get_cx_ref::<usize>(), 2);

    *cx.get_cx_mut::<usize>() = 3;
    assert_eq!(*cx.get_cx_ref::<usize>(), 3);

    cx.unregister_cx::<usize>();
    assert_eq!(*cx.get_cx_ref::<usize>(), 1);

    cx.unregister_cx::<usize>();
    assert!(cx.get_cx_ptr::<usize>().is_none());

    cx.message.put(a);
    assert_eq!(cx.message.take::<usize>(), Some(1));
    assert!(cx.message.take::<usize>().is_none());
  }
}
