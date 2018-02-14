extern crate log;
extern crate env_logger;
extern crate actix;
extern crate actix_remote;
extern crate serde_json;
extern crate structopt;
#[macro_use] extern crate structopt_derive;

use actix_remote::*;
use structopt::StructOpt;


#[derive(StructOpt, Debug)]
struct Cli {
    /// Network address
    addr: String,
    /// Network node address
    node: Option<String>,
}

fn main() {
    ::std::env::set_var("RUST_LOG", "actix_remote=info");
    let _ = env_logger::init();

    // cmd arguments
    let args = Cli::from_args();
    let addr = args.addr.to_lowercase().trim().to_owned();
    let node = args.node.map(|n| n.to_lowercase().trim().to_owned());

    let sys = actix::System::new("remote-example");

    Network::new(addr).unwrap()
        .add_node(node)
        .start();

    let _ = sys.run();
}
