#![allow(dead_code, unused_variables)]
use std::marker::PhantomData;

use actix::prelude::*;
use actix::dev::{MessageResponse, ResponseChannel};
use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use futures::Future;
use futures::sync::oneshot::Receiver;
use futures::unsync::oneshot::{self, Sender};

use actix::dev::{MessageRecipientTransport, SendError};

use msgs;
use node::NetworkNode;
use remote::{Remote, RemoteMessage};

pub trait RemoteMessageHandler: Send + Sync {
    fn handle(&self, msg: String, sender: Sender<String>);
}

/// Remote message handler
pub(crate)
struct RemoteRecipient<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub recipient: Recipient<Syn, M>,
}

impl<M> RemoteMessageHandler for RemoteRecipient<M>
    where M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    fn handle(&self, msg: String, sender: Sender<String>) {
        let msg = serde_json::from_slice::<M>(msg.as_ref()).unwrap();
        Arbiter::handle().spawn(
            self.recipient.send(msg).then(|res| {
                match res {
                    Ok(res) => {
                        let body = serde_json::to_string(&res).unwrap();
                        let _ = sender.send(body);
                    },
                    Err(e) => (),
                }
                Ok::<_, ()>(())
            }))
    }
}

/// Recipient proxy actor
pub(crate)
struct RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    m: PhantomData<M>,
    nodes: Vec<Addr<Unsync, NetworkNode>>,
}

impl<M> RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub fn new() -> Self {
        RecipientProxy{m: PhantomData, nodes: Vec::new()}
    }
}

/// Actor definition
impl<M> Actor for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    type Context = Context<Self>;
}

impl<M> msgs::NodeOperations for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned {}

/// Handler for proxied message
impl<M> Handler<M> for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    type Result = RecipientProxyResult<M>;

    fn handle(&mut self, msg: M, ctx: &mut Context<Self>) -> RecipientProxyResult<M> {
        let (tx, rx) = oneshot::channel();
        let body = serde_json::to_string(&msg).unwrap();
        if let Some(node) = self.nodes.first() {
            node.do_send(msgs::SendRemoteMessage{
                type_id: M::type_id().to_string(), data: body, tx: tx});
        }
        RecipientProxyResult{m: PhantomData, rx: rx}
    }
}

impl<M> Handler<msgs::TypeSupported> for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    type Result = ();

    fn handle(&mut self, msg: msgs::TypeSupported, ctx: &mut Context<Self>) {
        self.nodes.push(msg.node);
        debug!("type support {:?} {:?}", msg.type_id, M::type_id());
    }
}

impl<M> Handler<msgs::NodeGone> for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    type Result = ();

    fn handle(&mut self, msg: msgs::NodeGone, ctx: &mut Context<Self>) {
        unimplemented!()
    }
}

/// Proxied message result
pub struct RecipientProxyResult<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    m: PhantomData<M>,
    rx: oneshot::Receiver<String>,
}

impl<M> MessageResponse<RecipientProxy<M>, M> for RecipientProxyResult<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut Context<RecipientProxy<M>>, tx: Option<R>) {
        Arbiter::handle().spawn(
            self.rx
                .map_err(|e| ())
                .and_then(move |msg| {
                    let msg = serde_json::from_slice::<M::Result>(msg.as_ref()).unwrap();
                    if let Some(tx) = tx {
                        let _ = tx.send(msg);
                    }
                    Ok(())
                })
        );
    }
}

/// Sender proxy
pub struct RecipientProxySender<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    m: PhantomData<M>,
    tx: Addr<Syn, RecipientProxy<M>>,
}

use remote::RemoteRecipientRequest;

impl<M> RecipientProxySender<M>
    where M: RemoteMessage,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub(crate) fn new(addr: Addr<Syn, RecipientProxy<M>>) -> RecipientProxySender<M> {
        RecipientProxySender{m: PhantomData, tx: addr}
    }

    pub fn do_send(&self, msg: M) -> Result<(), SendError<M>> {
        self.tx.do_send(msg);
        Ok(())
    }

    pub fn try_send(&self, msg: M) -> Result<(), SendError<M>> {
        self.tx.try_send(msg)
    }

    pub fn send(&self, msg: M) -> RemoteRecipientRequest<Remote, M> {
        RemoteRecipientRequest::new(self.tx.send(msg))
    }
}

impl<M> MessageRecipientTransport<Remote, M> for RecipientProxySender<M>
    where M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    fn send(&self, msg: M) -> Result<Receiver<M::Result>, SendError<M>> {
        // RecipientProxySender::send(self, msg)
        unimplemented!()
    }
}

impl<M> Clone for RecipientProxySender<M>
    where M: RemoteMessage, M::Result: Send + Serialize + DeserializeOwned,
{
    fn clone(&self) -> Self {
        RecipientProxySender {m: PhantomData, tx: self.tx.clone()}
    }
}
