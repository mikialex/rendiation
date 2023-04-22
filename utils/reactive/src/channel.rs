use std::{fmt, result::Result};

use crate::*;

#[derive(Debug)]
pub struct Receiver<T> {
  inner: Arc<Mutex<(Option<T>, Option<Waker>)>>,
}

impl<T> Stream for Receiver<T> {
  type Item = T;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    if let Ok(mut inner) = self.inner.lock() {
      inner.1 = cx.waker().clone().into();
      if inner.0.is_some() {
        // maybe check is_some first will avoid unnecessary move
        let value = inner.0.take().unwrap();
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

/// The updating-half of the single value channel.
#[derive(Debug)]
pub struct Updater<T> {
  inner: Weak<Mutex<(Option<T>, Option<Waker>)>>,
}

impl<T> Clone for Updater<T> {
  fn clone(&self) -> Self {
    Updater {
      inner: Weak::clone(&self.inner),
    }
  }
}

/// An error returned from the [`Updater::update`](struct.Updater.html#method.update) function.
/// Indicates that the paired [`Receiver`](struct.Receiver.html) has been dropped.
///
/// Contains the value that had been passed into
/// [`Updater::update`](struct.Updater.html#method.update)
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct NoReceiverError<T>(pub T);

impl<T> fmt::Debug for NoReceiverError<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "NoReceiverError")
  }
}

impl<T> fmt::Display for NoReceiverError<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "receiver has been dropped")
  }
}

impl<T> std::error::Error for NoReceiverError<T> {}

impl<T> Updater<T> {
  /// Updates the latest value in this channel, to be accessed the next time
  ///
  /// This call will fail with [`NoReceiverError`](struct.NoReceiverError.html) if the receiver
  /// has been dropped.
  pub fn update(&self, value: T) -> Result<(), NoReceiverError<T>> {
    match self.inner.upgrade() {
      Some(mutex) => {
        let inner = &mut mutex.lock().unwrap();
        inner.0 = Some(value);
        if let Some(waker) = &inner.1 {
          waker.wake_by_ref()
        }
        Ok(())
      }
      None => Err(NoReceiverError(value)),
    }
  }

  /// Returns true if the receiver has been dropped. Thus indicating any following call to
  /// [`Updater::update`](struct.Updater.html#method.update) would fail.
  pub fn has_no_receiver(&self) -> bool {
    self.inner.upgrade().is_none()
  }
}

pub fn single_value_channel<T>() -> (Receiver<Option<T>>, Updater<Option<T>>) {
  let receiver = Receiver {
    inner: Arc::new(Mutex::new((None, None))),
  };
  let updater = Updater {
    inner: Arc::downgrade(&receiver.inner),
  };
  (receiver, updater)
}
