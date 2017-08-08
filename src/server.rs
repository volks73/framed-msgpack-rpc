// Portions of this were taken from the [rmp-rpc](https://github.com/little-dude/rmp-rpc) project.

//! Building blocks for building a `Framed-MessagePack-RPC` server.

use codec::Codec;
use futures::{Async, BoxFuture, Future, Poll, Sink, Stream};
use message::{Message, Response};
use rmpv::Value;
use std::io;
use std::collections::HashMap;
use std::error::Error;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;

/// The `Handler` trait defines how the server handles the requests and notifications it receives.
pub trait Handler: Clone {
    type Error: Error;
    type T: Into<Value>;
    type E: Into<Value>;

    /// Handle a `MessagePack-RPC` request.
    ///
    /// The framing is handled automatically by the codec.
    fn handle_request(&mut self, method: &str, params: &[Value]) -> BoxFuture<Result<Self::T, Self::E>, Self::Error>;

    /// Handle a `MessagePack-RPC` notification.
    ///
    /// The framing is handled automatically by the codec.
    fn handle_notification(&mut self, method: &str, params: &[Value]) -> BoxFuture<(), Self::Error>;
}

/// A Framed-Msgpack-RPC server that can handle requests and notifications.
pub struct Server<T: AsyncRead + AsyncWrite, H: Handler> {
    handler: H,
    io: Framed<T, Codec>,
    request_tasks: HashMap<u32, BoxFuture<Result<H::T, H::E>, H::Error>>,
    notification_tasks: Vec<BoxFuture<(), H::Error>>,
}

impl<T: AsyncRead + AsyncWrite + 'static, H: Handler + 'static> Server<T, H> {
    /// Creates a new `Server`.
    pub fn new(handler: H, io: T) -> Self {
        Server {
            handler: handler,
            io: io.framed(Codec::new()),
            request_tasks: HashMap::new(),
            notification_tasks: Vec::new(),
        }
    }

    fn handle_msg(&mut self, msg: Message) {
        match msg {
            Message::Request(request) => {
                let method = request.method.as_str();
                let params = request.params;
                let response = self.handler.handle_request(method, &params);
                self.request_tasks.insert(request.id, response);
            }
            Message::Notification(notification) => {
                let method = notification.method.as_str();
                let params = notification.params;
                let outcome = self.handler.handle_notification(method, &params);
                self.notification_tasks.push(outcome);
            }
            Message::Response(_) => {
                return;
            }
        }
    }

    fn process_notifications(&mut self) {
        let mut done = vec![];
        for (idx, task) in self.notification_tasks.iter_mut().enumerate() {
            match task.poll().unwrap() {
                Async::Ready(_) => done.push(idx),
                Async::NotReady => continue,
            }
        }
        for idx in done.iter().rev() {
            self.notification_tasks.remove(*idx);
        }
    }

    fn process_requests(&mut self) {
        let mut done = vec![];
        for (id, task) in &mut self.request_tasks {
            match task.poll().unwrap() {
                Async::Ready(response) => {
                    let msg = Message::Response(Response {
                        id: *id,
                        result: response.map(|v| v.into()).map_err(|e| e.into()),
                    });
                    done.push(*id);
                    if !self.io.start_send(msg).unwrap().is_ready() {
                        panic!("the sink is full")
                    }
                }
                Async::NotReady => continue,
            }
        }
        for idx in done.iter_mut().rev() {
            let _ = self.request_tasks.remove(idx);
        }
    }
}

impl<T: AsyncRead + AsyncWrite + 'static, H: Handler + 'static> Future for Server<T, H> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let item = try_ready!(self.io.poll());
        match item {
            Some(msg) => self.handle_msg(msg),
            None => {},
        }
        self.process_notifications();
        self.process_requests();
        self.io.poll_complete().unwrap();
        Ok(Async::NotReady)
    }
}

