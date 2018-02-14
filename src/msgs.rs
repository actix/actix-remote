#![allow(dead_code)]

use std::net;
use actix::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

use remote::RemoteMessage;

#[derive(Message)]
pub struct RegisterNode {
    pub addr: net::SocketAddr,
}

#[derive(Message)]
pub struct ReconnectNode;

#[derive(Message)]
pub struct NodeConnected(pub String);

pub struct RegisterRecipient<M: RemoteMessage + 'static>
    where M::Result: Send + Serialize + DeserializeOwned
{
    provider: Recipient<Syn, M>
}
