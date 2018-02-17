#![allow(dead_code)]

use std::{net, io};
use std::sync::Arc;
use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;
use futures::sync::mpsc::Receiver;
use futures::unsync::oneshot::Sender;

use actix::{Actor, Addr, Handler, Message, Unsync};

use node::NetworkNode;
use remote::RemoteMessage;
use recipient::RemoteMessageHandler;

#[derive(Message)]
pub struct RegisterNode {
    pub addr: net::SocketAddr,
}

#[derive(Message)]
pub struct ReconnectNode;

#[derive(Message)]
pub struct NodeConnected(pub String);

#[derive(Message, Clone)]
pub struct NodeSupportedTypes {
    pub node: String,
    pub types: Vec<String>,
}

#[derive(Message)]
pub struct StopWorker;

#[derive(Message)]
pub struct WorkerDisconnected(pub usize);

#[derive(Message)]
pub struct RegisterRecipient(pub &'static str, pub Arc<RemoteMessageHandler>);


#[derive(Message)]
pub(crate) struct GetRecipient<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub rx: Receiver<M>,
}

#[derive(Message)]
pub(crate) struct NodeGone(pub String);

#[derive(Message)]
pub(crate) struct TypeSupported{
    pub type_id: String,
    pub node: Addr<Unsync, NetworkNode>}

pub(crate) trait NodeOperations: Actor + Handler<NodeGone> + Handler<TypeSupported> {}


pub(crate) struct SendRemoteMessage{
    pub type_id: String,
    pub data: String,
    pub tx: Sender<String>,
}

impl Message for SendRemoteMessage {
    type Result = Result<String, io::Error>;
}
