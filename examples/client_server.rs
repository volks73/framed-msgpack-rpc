extern crate env_logger;
extern crate framed_msgpack_rpc;
extern crate futures;
extern crate rmpv;
extern crate tokio_core;

use framed_msgpack_rpc::server::{Handler, Server};
use framed_msgpack_rpc::client::Client;
use futures::{future, BoxFuture, Future, Stream};
use rmpv::Value;
use std::marker::Send;
use std::io;
use std::thread;
use std::time::Duration;
use std::net::SocketAddr;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;

#[derive(Clone)]
pub struct ExampleHandler;

fn box_ok<T: Send + 'static, E: Send + 'static>(t: T) -> BoxFuture<T, E> {
    Box::new(future::ok(t))
}

impl Handler for ExampleHandler {
    type Error = io::Error;
    type T = String;
    type E = String;

    fn handle_request(&mut self, method: &str, params: &[Value]) -> BoxFuture<Result<Self::T, Self::E>, Self::Error> {
        if method != "sayHello" {
            return box_ok(Err(format!("Unknown method '{}'", method)));
        }

        if params.len() != 1 {
            return box_ok(Err(format!("Expected 1 argument for method 'sayHello', got {}", params.len())));
        }

        if let Value::String(ref string) = params[0] {
            if let Some(name) = string.as_str() {
                return box_ok(Ok(format!("Hello {}!", name)));
            }
        }
        box_ok(Err("Invalid argument".into()))
    }

    fn handle_notification(&mut self, method: &str, _params: &[Value]) -> BoxFuture<(), Self::Error> {
        box_ok(println!("{}", method))
    }
}

fn main() {
    env_logger::init().unwrap();
    let address: SocketAddr = "127.0.0.1:12345".parse().unwrap();

    // Start the server
    thread::spawn(move || {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let listener = TcpListener::bind(&address, &handle).unwrap();
        core.run(listener.incoming().for_each(|(stream, _address)| {
            let proto = Server::new(ExampleHandler.clone(), stream);
            handle.spawn(proto.map_err(|_| ()));
            Ok(())
        })).unwrap()
    });

    // Allow some time for the server to start
    thread::sleep(Duration::from_millis(100));

    // Start the client
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let _ = core.run(
        Client::connect(&address, &handle)
            .or_else(|e| {
                println!("Connection to server failed: {}", e);
                Err(())
            })
            .and_then(|client| {
                client.request("sayHello", &["World".into()])
                    .and_then(|response| { 
                        println!("{:?}", response); 
                        Ok(client)
                    })
            }) 
            .and_then(|client| {
                client.notify("This is a notification", &[])
                    .and_then(|_| {
                        Ok(client)
                    })
            })
            .and_then(|client| {
                client.request("sayGoodbye", &[])
                    .and_then(|response| { 
                        println!("{:?}", response);
                        Ok(())
                    })
            })
    );
}

