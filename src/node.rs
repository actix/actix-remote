use std::io;
use std::cell::Cell;
use std::sync::Arc;
use backoff::ExponentialBackoff;
use backoff::backoff::Backoff;
use tokio_core::net::TcpStream;
use tokio_io::AsyncRead;
use tokio_io::io::WriteHalf;
use tokio_io::codec::FramedRead;
use actix::prelude::*;

use msgs;
use network::Network;
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
    net: Addr<Unsync, Network>,
    addr: String,
    inner: NodeInformation,
    backoff: ExponentialBackoff,
    framed: Option<actix::io::FramedWrite<WriteHalf<TcpStream>, NetworkClientCodec>>,
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
                    let mut framed = actix::io::FramedWrite::new(w, NetworkClientCodec::default(), ctx);
                    framed.write(Request::Handshake(act.addr.clone()));
                    act.framed = Some(framed);
                    act.backoff.reset();
                    act.inner.set_status(NodeStatus::Ok);
                    ctx.add_stream(FramedRead::new(r, NetworkClientCodec::default()));
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
    pub fn new(addr: String, net: Addr<Unsync, Network>, info: NodeInformation) -> NetworkNode {
        info!("New network node: {}", addr);
        NetworkNode {net: net,
                     addr: addr,
                     inner: info,
                     framed: None,
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
    fn handle(&mut self, _: Response, _ctx: &mut Self::Context) {
        // println!("test: {:?}", msg);
    }
}

/// Reconnect node if required
impl Handler<msgs::ReconnectNode> for NetworkNode {
    type Result = ();

    fn handle(&mut self, _: msgs::ReconnectNode, ctx: &mut Context<Self>) {
        self.stop_actor(ctx);
    }
}
