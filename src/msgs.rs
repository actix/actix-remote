#![allow(dead_code)]

use std::{net, io};
use std::sync::Arc;
use serde::Serialize;
use serde::de::DeserializeOwned;
use futures::sync::mpsc::Receiver;
use futures::unsync::oneshot::Sender;

use actix::{Actor, Addr, Handler, Message, Unsync};

use node::NetworkNode;
use remote::RemoteMessage;
use recipient::RemoteMessageHandler;

#[derive(Message)]
pub(crate) struct RegisterNode {
    pub addr: net::SocketAddr,
}

#[derive(Message)]
pub(crate) struct ReconnectNode;

#[derive(Message)]
pub(crate) struct NodeConnected(pub String);

/// NetworkNode notifies world.
/// New remote recipient is available.
#[derive(Message, Clone)]
pub(crate) struct NodeSupportedTypes {
    pub node: String,
    pub types: Vec<String>,
}

#[derive(Message)]
pub(crate) struct WorkerDisconnected(pub usize);

/// Register new recipient provider
#[derive(Message, Clone)]
pub struct ProvideRecipient{
    pub type_id: &'static str,
    pub handler: Arc<RemoteMessageHandler>}

#[derive(Message)]
pub(crate) struct GetRecipient<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub rx: Receiver<M>,
}

#[derive(Message)]
pub(crate) struct NodeGone(pub String);

/// World sends this message to RecipientProxy.
/// Notifies about new node with support of specific type_id.
#[derive(Message)]
pub(crate) struct TypeSupported {
    pub type_id: String,
    pub node_id: String,
    pub node: Addr<Unsync, NetworkNode> }

pub(crate) trait NodeOperations: Actor + Handler<NodeGone> + Handler<TypeSupported> {}


pub(crate) struct SendRemoteMessage{
    pub type_id: String,
    pub data: String,
    pub tx: Sender<String>,
}

impl Message for SendRemoteMessage {
    type Result = Result<String, io::Error>;
}

//===================================
// Worker messages
//===================================

/// Stop worker
#[derive(Message)]
pub(crate) struct StopWorker;
