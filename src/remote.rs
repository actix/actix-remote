use std::time::Duration;
use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;
use futures::{Async, Future, Poll};
use futures::sync::oneshot::Receiver;
use tokio_core::reactor::Timeout;

use actix::prelude::*;
use actix::dev::{Message, MessageRecipient,
                 SendError, MailboxError, MessageRecipientTransport};

use recipient::RecipientProxySender;


pub trait RemoteMessage: Message + Send + Serialize + DeserializeOwned
    where Self::Result: Send + Serialize + DeserializeOwned
{
    fn type_id() -> &'static str;
}

pub struct Remote;

impl<M> MessageRecipient<M> for Remote
    where M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    type Envelope = RemoteMessageEnvelope<M>;
    type Transport = RecipientProxySender<M>;

    type SendError = SendError<M>;
    type MailboxError = MailboxError;
    type Receiver = Receiver<M::Result>;
    type ReceiverFuture = RemoteRecipientRequest<Self, M>;

    fn do_send(tx: &Self::Transport, msg: M) -> Result<(), SendError<M>> {
        tx.do_send(msg)
    }

    fn try_send(tx: &Self::Transport, msg: M) -> Result<(), SendError<M>> {
        tx.try_send(msg)
    }

    fn send(tx: &Self::Transport, msg: M) -> RemoteRecipientRequest<Self, M> {
        tx.send(msg)
    }

    fn clone(tx: &Self::Transport) -> Self::Transport {
        tx.clone()
    }
}

pub struct RemoteMessageEnvelope<M: RemoteMessage>
    where M::Result: Send + Serialize + DeserializeOwned
{
    msg: M,
}

impl<M: RemoteMessage> RemoteMessageEnvelope<M>
    where M::Result: Send + Serialize + DeserializeOwned
{
    pub fn into_inner(self) -> M {
        self.msg
    }
}

impl<M: RemoteMessage> From<M> for RemoteMessageEnvelope<M>
    where M::Result: Send + Serialize + DeserializeOwned
{
    fn from(msg: M) -> RemoteMessageEnvelope<M> {
        RemoteMessageEnvelope{msg: msg}
    }
}

use recipient::RecipientProxy;

/// `RecipientRequest` is a `Future` which represents asynchronous message sending process.
#[must_use = "future do nothing unless polled"]
pub struct RemoteRecipientRequest<T, M>
    where T: MessageRecipient<M>,
          M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    rx: actix::dev::Request<Syn, RecipientProxy<M>, M>,
    timeout: Option<Timeout>,
    _t: PhantomData<T>,
}

impl<T, M> RemoteRecipientRequest<T, M>
    where T: MessageRecipient<M, MailboxError=MailboxError>,
          M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    pub(crate) fn new(rx: actix::dev::Request<Syn, RecipientProxy<M>, M>)
                      -> RemoteRecipientRequest<T, M>
    {
        RemoteRecipientRequest{rx: rx, timeout: None, _t: PhantomData}
    }

    /// Set message delivery timeout
    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(Timeout::new(dur, Arbiter::handle()).unwrap());
        self
    }

    fn poll_timeout(&mut self) -> Poll<M::Result, MailboxError> {
        if let Some(ref mut timeout) = self.timeout {
            match timeout.poll() {
                Ok(Async::Ready(())) => Err(MailboxError::Timeout),
                Ok(Async::NotReady) => Ok(Async::NotReady),
                Err(_) => unreachable!()
            }
        } else {
            Ok(Async::NotReady)
        }
    }
}

impl<T, M> Future for RemoteRecipientRequest<T, M>
    where T: MessageRecipient<M, SendError=SendError<M>, MailboxError=MailboxError>,
          M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    type Item = M::Result;
    type Error = T::MailboxError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.rx.poll() {
            Ok(Async::Ready(item)) => Ok(Async::Ready(item)),
            Ok(Async::NotReady) => {
                self.poll_timeout()
            }
            Err(_) => Err(MailboxError::Closed),
        }
    }
}
