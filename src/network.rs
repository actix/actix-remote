use std::{io, net};
use std::collections::HashMap;

use actix::prelude::*;
use tokio_core::net::TcpListener;
use tokio_io::{AsyncRead, AsyncWrite};

use msgs;
use utils;
use worker::NetworkWorker;
use node::{NetworkNode, NodeInformation};

pub struct Network {
    addr: String,
    addrs: HashMap<String, NodeInformation>,
    nodes: HashMap<String, Addr<Unsync, NetworkNode>>,
    sockets: HashMap<net::SocketAddr, net::TcpListener>,
}

impl Actor for Network {
    type Context = Context<Self>;
}

impl Network {
    pub fn new(addr: String) -> io::Result<Network> {
        let net = Network{addr: addr.clone(),
                          addrs: HashMap::new(),
                          nodes: HashMap::new(),
                          sockets: HashMap::new()};
        Ok(net.bind(addr)?)
    }

    /// The socket address to bind
    ///
    /// To mind multiple addresses this method can be call multiple times.
    pub fn bind<S: net::ToSocketAddrs>(mut self, addr: S) -> io::Result<Self> {
        let mut err = None;
        let mut succ = false;
        for addr in addr.to_socket_addrs()? {
            match utils::tcp_listener(addr, 256) {
                Ok(lst) => {
                    succ = true;
                    self.sockets.insert(lst.local_addr().unwrap(), lst);
                },
                Err(e) => err = Some(e),
            }
        }

        if !succ {
            if let Some(e) = err.take() {
                Err(e)
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "Can not bind to address."))
            }
        } else {
            Ok(self)
        }
    }

    /// Register network node
    pub fn add_node<S: Into<String>>(mut self, addr: Option<S>) -> Self {
        addr.map(|addr| {
            let addr = addr.into();
            self.addrs.insert(addr.clone(), NodeInformation::new(addr));
        });
        self
    }

    /// Create network nodes, and start listening for incoming connections
    pub fn start(mut self) -> Addr<Syn, Self> {
        if self.sockets.is_empty() {
            panic!("Network::bind() has to be called before start()");
        } else {
            let addrs: Vec<(net::SocketAddr, net::TcpListener)> =
                self.sockets.drain().collect();

            // start network
            Actor::create(move |ctx| {
                let h = Arbiter::handle();

                // start workers
                for (addr, sock) in addrs {
                    info!("Starting actix remote server on {}", addr);
                    let lst = TcpListener::from_listener(sock, &addr, h)
                        .unwrap();
                    ctx.add_stream(lst.incoming());
                }

                for info in self.addrs.values() {
                    let net = ctx.address();
                    let info2 = info.clone();
                    let addr2 = self.addr.clone();
                    let node: Addr<Unsync, _> =
                        Supervisor::start(move |_| NetworkNode::new(addr2, net, info2));
                    self.nodes.insert(info.address().to_string(), node);
                }

                self
            })
        }
    }
}

/// Handle client connection
impl<T, U> StreamHandler<(T, U), io::Error> for Network
    where T: AsyncRead + AsyncWrite + 'static
{
    fn handle(&mut self, msg: (T, U), ctx: &mut Context<Self>) {
        NetworkWorker::start(msg.0, ctx.address());
    }
}

/// New client connection, create new downstream connection or re-connect existing
impl Handler<msgs::NodeConnected> for Network {
    type Result = ();

    fn handle(&mut self, msg: msgs::NodeConnected, ctx: &mut Context<Self>) {
        if let Some(node) = self.nodes.get(&msg.0) {
            node.do_send(msgs::ReconnectNode);
            return
        }

        let nw = self.addr.clone();
        let addr = msg.0.clone();
        let net = ctx.address();
        let info = NodeInformation::new(msg.0.clone());
        let node: Addr<Unsync, _> =
            Supervisor::start(move |_| NetworkNode::new(nw, net, info));
        self.nodes.insert(addr, node);
    }
}
