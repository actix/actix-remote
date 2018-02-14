use std::io;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::io::WriteHalf;
use tokio_io::codec::FramedRead;
use actix::prelude::*;

use msgs::NodeConnected;
use network::Network;
use protocol::{Request, Response, NetworkServerCodec};

/// Worker accepts messages from other network hosts and
/// pass them to local recipients
pub struct NetworkWorker<T> where T: AsyncRead + AsyncWrite {
    net: Addr<Unsync, Network>,
    framed: actix::io::FramedWrite<WriteHalf<T>, NetworkServerCodec>,
}

impl<T> NetworkWorker<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    pub fn start(io: T, net: Addr<Unsync, Network> ) -> Addr<Unsync, Self> {
        Actor::create(move |ctx| {
            let (r, w) = io.split();
            ctx.add_stream(FramedRead::new(r, NetworkServerCodec::default()));
            let mut framed = actix::io::FramedWrite::new(w, NetworkServerCodec::default(), ctx);
            framed.write(Response::Handshake);
            NetworkWorker{net: net, framed: framed}
        })
    }
}

impl<T> Actor for NetworkWorker<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    type Context = Context<Self>;
}

impl<T> actix::io::WriteHandler<io::Error> for NetworkWorker<T>
    where T: AsyncRead + AsyncWrite + 'static {
}

impl<T> StreamHandler<Request, io::Error> for NetworkWorker<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    fn error(&mut self, _err: io::Error, _ctx: &mut Self::Context) -> Running {
        Running::Stop
    }

    /// This is main event loop for client connection
    fn handle(&mut self, msg: Request, _ctx: &mut Self::Context) {
        match msg {
            Request::Handshake(addr) => self.net.do_send(NodeConnected(addr)),
            _ => {
                println!("test: {:?}", msg);
            }
        }
    }
}
