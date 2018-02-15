use std::io;
use std::sync::Arc;
use std::collections::HashMap;

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::io::WriteHalf;
use tokio_io::codec::FramedRead;
use actix::prelude::*;

use msgs;
use msgs::NodeConnected;
use world::World;
use recipient::RemoteMessageHandler;
use protocol::{Request, Response, NetworkServerCodec};

/// Worker accepts messages from other network hosts and
/// pass them to local recipients
pub struct NetworkWorker<T> where T: AsyncRead + AsyncWrite {
    id: usize,
    net: Addr<Unsync, World>,
    handlers: HashMap<&'static str, Arc<RemoteMessageHandler>>,
    framed: actix::io::FramedWrite<WriteHalf<T>, NetworkServerCodec>,
}

impl<T> NetworkWorker<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    pub fn start(id: usize, io: T,
                 handlers: HashMap<&'static str, Arc<RemoteMessageHandler>>,
                 net: Addr<Unsync, World>) -> Addr<Unsync, Self>
    {
        Actor::create(move |ctx| {
            let (r, w) = io.split();

            // read side of the connection
            ctx.add_stream(FramedRead::new(r, NetworkServerCodec::default()));

            // write side of the connection
            let mut framed = actix::io::FramedWrite::new(w, NetworkServerCodec::default(), ctx);
            framed.write(Response::Handshake);

            // send list of supported messages
            framed.write(Response::Supported(
                handlers.keys().map(|s| s.to_string()).collect()));
            NetworkWorker{id: id, net: net, handlers: handlers, framed: framed}
        })
    }
}

impl<T> Actor for NetworkWorker<T> where T: AsyncRead + AsyncWrite + 'static {
    type Context = Context<Self>;
}

impl<T> actix::io::WriteHandler<io::Error> for NetworkWorker<T>
    where T: AsyncRead + AsyncWrite + 'static {
}

impl<T> StreamHandler<Request, io::Error> for NetworkWorker<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    fn finished(&mut self, ctx: &mut Self::Context) {

        self.net.do_send(msgs::WorkerDisconnected(self.id));
        ctx.stop();
    }
    
    /// This is main event loop for client connection
    fn handle(&mut self, msg: Request, _ctx: &mut Self::Context) {
        match msg {
            Request::Handshake(addr) => self.net.do_send(NodeConnected(addr)),
            _ => {
                println!("CLIENT REQ: {:?}", msg);
            }
        }
    }
}

/// World is dead
impl<T> Handler<msgs::StopWorker> for NetworkWorker<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    type Result = ();

    fn handle(&mut self, _: msgs::StopWorker, ctx: &mut Self::Context) {
        ctx.stop();
    }
}
