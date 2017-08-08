extern crate framed_msgpack_rpc;
extern crate futures;
extern crate rmpv;
extern crate tokio_core;

use framed_msgpack_rpc::server::{Handler, Server};
use futures::{BoxFuture, future, Future, Stream};
use rmpv::Value;
use std::io;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;

#[derive(Clone)]
struct ExampleHandler;

impl Handler for ExampleHandler {
    type Error = io::Error;
    type T = &'static str;
    type E = String;

    fn handle_request(&mut self, method: &str, _params: &[Value] ) -> BoxFuture<Result<Self::T, Self::E>, Self::Error> {
        Box::new(
            match method {
                "sayHello" => {
                    future::ok(Ok("Hello World!"))
                },
                method => {
                    future::ok(Err(format!("Unknown method '{}'", method)))
                },
            }
        )
    }

    fn handle_notification(&mut self, method: &str, _params: &[Value]) -> BoxFuture<(), Self::Error> {
        Box::new(future::ok(println!("{}", method)))
    }
}

fn main() {
    let mut core = Core::new().unwrap();
    let address = "127.0.0.1:12345".parse().unwrap();
    let handle = core.handle();
    let listener = TcpListener::bind(&address, &handle).unwrap();
    core.run(listener.incoming().for_each(|(stream, _address)| {
        let proto = Server::new(ExampleHandler.clone(), stream);
        handle.spawn(proto.map_err(|_| ()));
        Ok(())
    })).unwrap()
}

