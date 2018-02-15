#![allow(dead_code, unused_variables)]
use std::sync::Arc;
use std::marker::PhantomData;

use actix::prelude::*;
use actix::dev::{MessageResponse, ResponseChannel};
use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;
use futures::sync::oneshot::Receiver;

use actix::dev::{MessageRecipientTransport, SendError};

use msgs;
use world::World;
use node::NetworkNode;
use remote::{Remote, RemoteMessage};

pub trait RemoteMessageHandler: Send + Sync {
    fn handle(&self, msg: Bytes);
}

pub struct RemoteRecipient<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    recipient: Recipient<Syn, M>,
}

impl<M> RemoteRecipient<M>
    where M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned {

    pub fn register(world: &Addr<Syn, World>, recipient: Recipient<Syn, M>) {
        let r = RemoteRecipient{recipient: recipient};
        world.do_send(msgs::RegisterRecipient(M::type_id(), Arc::new(r)))
    }
}

impl<M> RemoteMessageHandler for RemoteRecipient<M>
    where M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    fn handle(&self, _msg: Bytes) {
        println!("REMOTE MESSAGE");
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

impl<M> Actor for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    type Context = Context<Self>;
}

impl<M> msgs::NodeOperations for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned {}

impl<M> Handler<M> for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    type Result = RecipientProxyResult<M>;

    fn handle(&mut self, msg: M, ctx: &mut Context<Self>) -> RecipientProxyResult<M> {
        unimplemented!()
    }
}

impl<M> Handler<msgs::TypeSupported> for RecipientProxy<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    type Result = ();

    fn handle(&mut self, msg: msgs::TypeSupported, ctx: &mut Context<Self>) {
        println!("new node");
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

pub struct RecipientProxyResult<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    msg: M::Result,
}

impl<M> MessageResponse<RecipientProxy<M>, M> for RecipientProxyResult<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut Context<RecipientProxy<M>>, tx: Option<R>) {
        unimplemented!()
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

impl<M> RecipientProxySender<M>
    where M: RemoteMessage,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub(crate) fn new(addr: Addr<Syn, RecipientProxy<M>>) -> RecipientProxySender<M> {
        RecipientProxySender{m: PhantomData, tx: addr}
    }

    pub fn do_send(&self, _msg: M) -> Result<(), SendError<M>> {
        unimplemented!()
    }

    pub fn try_send(&self, _msg: M) -> Result<(), SendError<M>> {
        unimplemented!()
    }

    pub fn send(&self, _msg: M) -> Result<Receiver<M::Result>, SendError<M>> {
        unimplemented!()
    }
}

impl<M> MessageRecipientTransport<Remote, M> for RecipientProxySender<M>
    where M: RemoteMessage + 'static, M::Result: Send + Serialize + DeserializeOwned
{
    fn send(&self, msg: M) -> Result<Receiver<M::Result>, SendError<M>> {
        RecipientProxySender::send(self, msg)
    }
}

impl<M> Clone for RecipientProxySender<M>
    where M: RemoteMessage, M::Result: Send + Serialize + DeserializeOwned,
{
    fn clone(&self) -> Self {
        RecipientProxySender {m: PhantomData, tx: self.tx.clone()}
    }
}
