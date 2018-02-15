extern crate log;
extern crate env_logger;
#[macro_use] extern crate actix;
extern crate actix_remote;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate structopt;
#[macro_use] extern crate structopt_derive;

use actix_remote::*;
use actix::prelude::*;
use structopt::StructOpt;


#[derive(Debug, Message, Serialize, Deserialize)]
struct TestMessage {
    msg: String,
}

impl RemoteMessage for TestMessage {
    fn type_id() -> &'static str {
        "TestMessage"
    }
}

struct MyActor {
    cnt: usize,
    recipient: Recipient<Remote, TestMessage>,
}

impl Actor for MyActor {
    type Context = Context<Self>;
}

impl Handler<TestMessage> for MyActor {
    type Result = ();

    fn handle(&mut self, msg: TestMessage, ctx: &mut Context<Self>) {
        println!("MSG: {:?}", msg);
    }
}


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

    let mut world = World::new(addr).unwrap().add_node(node);
    let recipient = world.get_recipient::<TestMessage>();
    let addr = world.start();

    let a: Addr<Unsync, _> = MyActor::create(move |ctx| {
        RemoteRecipient::register(
            &addr, ctx.address::<Addr<Syn, _>>().recipient());
        MyActor{cnt: 0, recipient}
    });

    let _ = sys.run();
}
