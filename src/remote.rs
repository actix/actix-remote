use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;
use futures::unsync::oneshot::Receiver;

use actix::dev::{Message, MessageRecipient, MessageRecipientTransport,
                 SendError, RecipientRequest};


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
    type Transport = RemoteSender<M>;
    type ResultReceiver = Receiver<M::Result>;

    fn do_send(tx: &Self::Transport, msg: M) -> Result<(), SendError<M>> {
        tx.do_send(msg)
    }

    fn try_send(tx: &Self::Transport, msg: M) -> Result<(), SendError<M>> {
        tx.try_send(msg)
    }

    fn send(tx: &Self::Transport, msg: M) -> RecipientRequest<Self, M> {
        match tx.send(msg) {
            Ok(rx) => RecipientRequest::new(Some(rx), None),
            Err(SendError::Full(msg)) =>
                RecipientRequest::new(None, Some((tx.clone(), msg))),
            Err(SendError::Closed(_)) =>
                RecipientRequest::new(None, None),
        }
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

pub struct RemoteSender<M: RemoteMessage>
    where M::Result: Send + Serialize + DeserializeOwned
{
    m: PhantomData<M>
}

impl<M> RemoteSender<M> where M: RemoteMessage, M::Result: Send + Serialize + DeserializeOwned
{
    fn do_send(&self, _msg: M) -> Result<(), SendError<M>> {
        unimplemented!()
    }

    fn try_send(&self, _msg: M) -> Result<(), SendError<M>> {
        unimplemented!()
    }

    fn send(&self, _msg: M) -> Result<Receiver<M::Result>, SendError<M>> {
        unimplemented!()
    }
}

impl<M> MessageRecipientTransport<Remote, M> for RemoteSender<M>
    where M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    fn send(&self, msg: M) -> Result<Receiver<M::Result>, SendError<M>> {
        RemoteSender::send(self, msg)
    }
}

impl<M> Clone for RemoteSender<M>
    where M: RemoteMessage, M::Result: Send + Serialize + DeserializeOwned,
{
    fn clone(&self) -> Self {
        RemoteSender {m: PhantomData}
    }
}
