use std::{io, net};
use net2::TcpBuilder;


pub fn tcp_listener(addr: net::SocketAddr, backlog: i32) -> io::Result<net::TcpListener> {
    let builder = match addr {
        net::SocketAddr::V4(_) => TcpBuilder::new_v4()?,
        net::SocketAddr::V6(_) => TcpBuilder::new_v6()?,
    };
    builder.bind(addr)?;
    builder.reuse_address(true)?;
    Ok(builder.listen(backlog)?)
}
