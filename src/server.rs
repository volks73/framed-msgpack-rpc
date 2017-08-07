// Portions of this were taken from the [rmp-rpc](https://github.com/little-dude/rmp-rpc) project.

use codec::Codec;
use futures::{Async, BoxFuture, Future, Poll, Sink, Stream};
use message::{Message, Response};
use rmpv::Value;
use std::io;
use std::collections::HashMap;
use std::error::Error;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;

/// The `Service` trait defines how the server handles the requests and notifications it receives.
pub trait Service {
    type Error: Error;
    type T: Into<Value>;
    type E: Into<Value>;

    /// Handle a `MessagePack-RPC` request
    fn handle_request(&mut self, method: &str, params: &[Value]) -> BoxFuture<Result<Self::T, Self::E>, Self::Error>;

    /// Handle a `MessagePack-RPC` notification
    fn handle_notification(&mut self, method: &str, params: &[Value]) -> BoxFuture<(), Self::Error>;
}

/// Since a new `Service` is created for each client, it is necessary to have a builder type that
/// implements the `ServiceBuilder` trait.
pub trait ServiceBuilder {
    type Service: Service + 'static;

    /// Creates the service.
    fn build(&self) -> Self::Service;
}

/// A MessagePack-RPC server that can handle requests and notifications.
pub struct Server<T: AsyncRead + AsyncWrite, S: Service> {
    service: S,
    io: Framed<T, Codec>,
    request_tasks: HashMap<u32, BoxFuture<Result<S::T, S::E>, S::Error>>,
    notification_tasks: Vec<BoxFuture<(), S::Error>>,
}

impl<T: AsyncRead + AsyncWrite + 'static, S: Service + 'static> Server<T, S> {
    /// Creates a new `Server`.
    pub fn new(service: S, io: T) -> Self {
        Server {
            service: service,
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
                let response = self.service.handle_request(method, &params);
                self.request_tasks.insert(request.id, response);
            }
            Message::Notification(notification) => {
                let method = notification.method.as_str();
                let params = notification.params;
                let outcome = self.service.handle_notification(method, &params);
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

    fn flush(&mut self) {
        self.io.poll_complete().unwrap();
    }
}

impl<T: AsyncRead + AsyncWrite + 'static, S: Service + 'static> Future for Server<T, S> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.io.poll().unwrap() {
                Async::Ready(Some(msg)) => self.handle_msg(msg),
                Async::Ready(None) => {
                    return Ok(Async::Ready(()));
                }
                Async::NotReady => break,
            }
        }
        self.process_notifications();
        self.process_requests();
        self.flush();
        Ok(Async::NotReady)
    }
}

