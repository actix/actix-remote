use std::io;
use std::cell::Cell;
use std::sync::Arc;
use std::collections::HashMap;
use backoff::ExponentialBackoff;
use backoff::backoff::Backoff;
use futures::unsync::oneshot;
use tokio_core::net::TcpStream;
use tokio_io::AsyncRead;
use tokio_io::io::WriteHalf;
use tokio_io::codec::FramedRead;
use actix::prelude::*;
use actix::prelude::{Response as ActixResponse};

use msgs;
use world::World;
use protocol::{Request, Response, NetworkClientCodec};


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NodeStatus {
    New,
    Ok,
    Connecting,
    Failed,
}

pub struct NodeInformation {
    inner: Arc<Inner>,
}

impl NodeInformation {
    pub fn new(addr: String) -> NodeInformation {
        NodeInformation{inner: Arc::new(
            Inner{addr: addr,
                  status: Cell::new(NodeStatus::New)}
        )}
    }

    pub fn address(&self) -> &str {
        self.inner.as_ref().addr.as_str()
    }

    pub fn status(&self) -> NodeStatus {
        self.inner.as_ref().status.get()
    }

    pub fn set_status(&self, status: NodeStatus) {
        self.inner.as_ref().status.set(status)
    }
}

impl Clone for NodeInformation {
    fn clone(&self) -> Self {
        NodeInformation {inner: Arc::clone(&self.inner)}
    }
}

struct Inner {
    addr: String,
    status: Cell<NodeStatus>,
}

/// NetworkNode - Actor responsible for network node
pub struct NetworkNode {
    mid: u64,
    world: Addr<Unsync, World>,
    addr: String,
    inner: NodeInformation,
    backoff: ExponentialBackoff,
    framed: Option<actix::io::FramedWrite<WriteHalf<TcpStream>, NetworkClientCodec>>,
    requests: HashMap<u64, oneshot::Sender<String>>,
}

impl Actor for NetworkNode {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.inner.set_status(NodeStatus::Connecting);

        // Connect to actix remote server
        actix::actors::Connector::from_registry()
            .send(actix::actors::Connect::host(self.inner.address().clone()))
            .into_actor(self)
            .map(|res, act, ctx| match res {
                Ok(stream) => {
                    info!("Connected to network node: {}", act.inner.address());

                    let (r, w) = stream.split();

                    // configure write side of the connection
                    let mut framed =
                        actix::io::FramedWrite::new(w, NetworkClientCodec::default(), ctx);
                    framed.write(Request::Handshake(act.addr.clone()));
                    act.framed = Some(framed);

                    // read side of the connection
                    ctx.add_stream(FramedRead::new(r, NetworkClientCodec::default()));

                    act.backoff.reset();
                    act.inner.set_status(NodeStatus::Ok);
                },
                Err(err) => act.restart(Some(err), ctx),
            })
            .map_err(|_, act, ctx| act.restart(None, ctx))
            .wait(ctx);
    }
}

impl Supervised for NetworkNode {
    fn restarting(&mut self, _: &mut Self::Context) {
        self.framed.take();
        self.inner.set_status(NodeStatus::Failed);
        //for tx in self.queue.drain(..) {
        //let _ = tx.send(Err(Error::Disconnected));
        //}
    }
}

impl actix::io::WriteHandler<io::Error> for NetworkNode {}

impl NetworkNode {
    pub fn new(addr: String, world: Addr<Unsync, World>, info: NodeInformation) -> NetworkNode {
        info!("New network node: {}", addr);
        NetworkNode {mid: 0,
                     world: world,
                     addr: addr,
                     inner: info,
                     framed: None,
                     requests: HashMap::new(),
                     backoff: ExponentialBackoff::default(),
        }
    }

    pub fn restart(&mut self, err: Option<actix::actors::ConnectorError>, ctx: &mut Context<Self>)
    {
        self.framed.take();
        self.inner.set_status(NodeStatus::Failed);

        if let Some(err) = err {
            error!("Can not connect to network node: {}, err: {}",
                   self.inner.address(), err);
            debug!("{:?}", err);
        } else {
            error!("Restart network node connection");
        }
        // re-connect with backoff time.
        // we stop currect context, supervisor will restart it.
        if let Some(timeout) = self.backoff.next_backoff() {
            ctx.run_later(timeout, |act, ctx| act.stop_actor(ctx));
        } else {
            self.stop_actor(ctx);
        }
    }

    fn stop_actor(&mut self, ctx: &mut Context<Self>) {
        if self.inner.status() == NodeStatus::Failed {
            ctx.stop()
        }
    }
}

impl StreamHandler<Response, io::Error> for NetworkNode
{
    fn error(&mut self, err: io::Error, _ctx: &mut Self::Context) -> Running {
        error!("Network node has been disconnected: {}, err: {}",
               self.inner.address(), err);
        Running::Stop
    }

    /// This is main event loop for server responses
    fn handle(&mut self, msg: Response, _ctx: &mut Self::Context) {
        match msg {
            Response::Supported(types) => {
                self.world.do_send(msgs::NodeSupportedTypes {
                    node: self.inner.address().to_string(),
                    types: types
                });
            },
            Response::Result(id, data) => {
                if let Some(tx) = self.requests.remove(&id) {
                    debug!("GOT REMOTE RESULT: {:?} {:?}", id, data);
                    let _ = tx.send(data);
                }
            },
            _ => (),
        }
    }
}

/// Reconnect node if required
impl Handler<msgs::ReconnectNode> for NetworkNode {
    type Result = ();

    fn handle(&mut self, _: msgs::ReconnectNode, ctx: &mut Context<Self>) {
        self.stop_actor(ctx);
    }
}


/// Send remote mesage
impl Handler<msgs::SendRemoteMessage> for NetworkNode {
    type Result = ActixResponse<String, io::Error>;

    fn handle(&mut self, msg: msgs::SendRemoteMessage, _: &mut Context<Self>) -> Self::Result {
        if let Some(ref mut framed) = self.framed {
            self.mid += 1;
            self.requests.insert(self.mid, msg.tx);
            framed.write(Request::Message(self.mid, msg.type_id, "1.0".to_string(), msg.data));
        }
        ActixResponse::reply(Err(io::Error::new(io::ErrorKind::Other, "test")))
    }
}
