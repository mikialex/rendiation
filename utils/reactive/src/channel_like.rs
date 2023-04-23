use crate::*;

pub trait ChannelLike<T> {
  type Sender: Clone + Send + Sync + 'static;
  type Receiver: Stream<Item = T> + Send + Sync + 'static;

  fn build() -> (Self::Sender, Self::Receiver);
  /// return if had sent successfully
  fn send(sender: &Self::Sender, message: T) -> bool;
  fn is_closed(sender: &Self::Sender) -> bool;
}

pub struct DefaultUnboundChannel;

impl<T: Send + Sync + 'static> ChannelLike<T> for DefaultUnboundChannel {
  type Sender = futures::channel::mpsc::UnboundedSender<T>;

  type Receiver = futures::channel::mpsc::UnboundedReceiver<T>;

  fn build() -> (Self::Sender, Self::Receiver) {
    futures::channel::mpsc::unbounded()
  }

  fn send(sender: &Self::Sender, message: T) -> bool {
    sender.unbounded_send(message).is_ok()
  }

  fn is_closed(sender: &Self::Sender) -> bool {
    sender.is_closed()
  }
}

pub struct DefaultSingleValueChannel;

impl<T: Send + Sync + 'static> ChannelLike<T> for DefaultSingleValueChannel {
  type Sender = crate::channel::Updater<T>;

  type Receiver = crate::channel::Receiver<T>;

  fn build() -> (Self::Sender, Self::Receiver) {
    crate::channel::single_value_channel()
  }

  fn send(sender: &Self::Sender, message: T) -> bool {
    sender.update(message).is_ok()
  }

  fn is_closed(sender: &Self::Sender) -> bool {
    sender.has_no_receiver()
  }
}
