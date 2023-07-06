use crate::*;

// todo share code with single value channel
#[derive(Debug)]
pub struct BatchReceiver<T> {
  inner: Arc<Mutex<(Vec<T>, Option<Waker>)>>,
}

impl<T> Stream for BatchReceiver<T> {
  type Item = Vec<T>;

  // todo check if we could early drop history if the sender dropped
  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    if let Ok(mut inner) = self.inner.lock() {
      inner.1 = cx.waker().clone().into();
      // check is_empty first to avoid unnecessary move
      if inner.0.is_empty() {
        let value = std::mem::take(&mut inner.0);
        Poll::Ready(Some(value))
        // check if sender has dropped
      } else if Arc::weak_count(&self.inner) == 0 {
        Poll::Ready(None)
      } else {
        Poll::Pending
      }
    } else {
      Poll::Ready(None)
    }
  }
}

#[derive(Debug)]
pub struct BatchSender<T> {
  inner: Weak<Mutex<(Vec<T>, Option<Waker>)>>,
}

impl<T> Drop for BatchSender<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      let inner = inner.lock().unwrap();
      if let Some(waker) = &inner.1 {
        waker.wake_by_ref()
      }
    }
  }
}

impl<T> Clone for BatchSender<T> {
  fn clone(&self) -> Self {
    BatchSender {
      inner: Weak::clone(&self.inner),
    }
  }
}

impl<T> BatchSender<T> {
  pub fn update(&self, value: T) -> Result<(), NoReceiverError<T>> {
    match self.inner.upgrade() {
      Some(mutex) => {
        let inner = &mut mutex.lock().unwrap();
        inner.0.push(value);
        if let Some(waker) = &inner.1 {
          waker.wake_by_ref()
        }
        Ok(())
      }
      None => Err(NoReceiverError(value)),
    }
  }

  pub fn has_no_receiver(&self) -> bool {
    self.inner.upgrade().is_none()
  }
}

pub fn batch_value_channel<T>() -> (BatchSender<T>, BatchReceiver<T>) {
  let receiver = BatchReceiver {
    inner: Arc::new(Mutex::new((Default::default(), None))),
  };
  let updater = BatchSender {
    inner: Arc::downgrade(&receiver.inner),
  };
  (updater, receiver)
}
