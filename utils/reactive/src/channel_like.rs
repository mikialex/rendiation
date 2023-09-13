use crate::*;

pub trait ChannelLike<T> {
  type Message;
  type Sender: Clone + Send + Sync + 'static;
  type Receiver: Stream<Item = Self::Message> + Send + Sync + 'static;

  fn build(&self) -> (Self::Sender, Self::Receiver);
  /// return if had sent successfully
  fn send(sender: &Self::Sender, message: T) -> bool;
  fn is_closed(sender: &Self::Sender) -> bool;
}

pub struct DefaultUnboundChannel;

impl<T: Send + Sync + 'static> ChannelLike<T> for DefaultUnboundChannel {
  type Message = T;
  type Sender = futures::channel::mpsc::UnboundedSender<T>;
  type Receiver = futures::channel::mpsc::UnboundedReceiver<T>;

  fn build(&self) -> (Self::Sender, Self::Receiver) {
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
  type Message = T;
  type Sender = crate::channel_single::SingleSender<T>;
  type Receiver = crate::channel_single::SingleReceiver<T>;

  fn build(&self) -> (Self::Sender, Self::Receiver) {
    crate::channel_single::single_value_channel()
  }

  fn send(sender: &Self::Sender, message: T) -> bool {
    sender.update(message).is_ok()
  }

  fn is_closed(sender: &Self::Sender) -> bool {
    sender.has_no_receiver()
  }
}

pub struct DefaultBatchChannel;

impl<T: Send + Sync + 'static> ChannelLike<T> for DefaultBatchChannel {
  type Message = Vec<T>;
  type Sender = crate::channel_batch::BatchSender<T>;
  type Receiver = crate::channel_batch::BatchReceiver<T>;

  fn build(&self) -> (Self::Sender, Self::Receiver) {
    crate::channel_batch::batch_value_channel()
  }

  fn send(sender: &Self::Sender, message: T) -> bool {
    sender.update(message).is_ok()
  }

  fn is_closed(sender: &Self::Sender) -> bool {
    sender.has_no_receiver()
  }
}
