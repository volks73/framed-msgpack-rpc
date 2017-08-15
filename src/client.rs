// Portions of this were taken from the [rmp-rpc](https://github.com/little-dude/rmp-rpc) project.

use codec::Codec;
use futures::{Async, Future, Poll, Sink, Stream};
use futures::sync::{mpsc, oneshot};
use message::{Message, Notification, Request};
use rmpv::Value;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_io::AsyncRead;
use tokio_io::codec::Framed;

pub struct Response {
    inner: oneshot::Receiver<Result<Value, Value>>,
}

impl Future for Response {
    type Item = Result<Value, Value>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll().map_err(|_| ())
    }
}

pub struct Ack {
    inner: oneshot::Receiver<()>,
}

impl Future for Ack {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll().map_err(|_| ())
    }
}

/// A client used to send request or notifications to a `Framed-MessagePack-RPC` server.
pub struct Client {
    requests_tx: mpsc::UnboundedSender<(Request, oneshot::Sender<Result<Value, Value>>)>,
    notifications_tx: mpsc::UnboundedSender<(Notification, oneshot::Sender<()>)>,
}

impl Client {
    /// Send a `Framed-MessagePack-RPC` request.
    pub fn request(&self, method: &str, params: &[Value]) -> Response {
        let request = Request {
            id: 0,
            method: method.to_owned(),
            params: Vec::from(params),
        };
        let (tx, rx) = oneshot::channel();
        // If send returns an Err, its because the other side has been dropped. By ignoring it,
        // we are just dropping the `tx`, which will mean the rx will return Canceled when
        // polled. In turn, that is translated into a BrokenPipe, which conveys the proper
        // error.
        let _ = mpsc::UnboundedSender::send(&self.requests_tx, (request, tx));
        Response { inner: rx }
    }

    /// Send a `Framed-MessagePack-RPC` notification.
    pub fn notify(&self, method: &str, params: &[Value]) -> Ack {
        let notification = Notification {
            method: method.to_owned(),
            params: Vec::from(params),
        };
        let (tx, rx) = oneshot::channel();
        let _ = mpsc::UnboundedSender::send(&self.notifications_tx, (notification, tx));
        Ack { inner: rx }
    }

    /// Connect the client to a remote `Framed-MessagePack-RPC` server.
    pub fn connect(addr: &SocketAddr, handle: &Handle) -> Connection {
        let (client_tx, client_rx) = oneshot::channel();
        let (error_tx, error_rx) = oneshot::channel();

        let connection = Connection {
            client_rx: client_rx,
            error_rx: error_rx,
            client_chan_cancelled: false,
            error_chan_cancelled: false,
        };

        let client = TcpStream::connect(addr, handle)
            .and_then(|stream| {
                let (requests_tx, requests_rx) = mpsc::unbounded();
                let (notifications_tx, notifications_rx) = mpsc::unbounded();
                let client = Client {
                    requests_tx: requests_tx,
                    notifications_tx: notifications_tx,
                };
                if client_tx.send(client).is_err() {
                    panic!("Failed to send client to connection");
                }
                Endpoint {
                    request_id: 0,
                    shutdown: false,
                    stream: stream.framed(Codec::new()),
                    requests_rx: requests_rx,
                    notifications_rx: notifications_rx,
                    pending_requests: HashMap::new(),
                    pending_notifications: Vec::new(),
                }
            })
            .or_else(|e| {
                if let Err(e) = error_tx.send(e) {
                    panic!("Failed to send client to connection: {:?}", e);
                }
                Err(())
            });

        handle.spawn(client);
        connection
    }
}

impl Future for Client {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::Ready(()))
    }
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Client {
            requests_tx: self.requests_tx.clone(),
            notifications_tx: self.notifications_tx.clone(),
        }
    }
}

struct Endpoint {
    stream: Framed<TcpStream, Codec>,
    request_id: u32,
    shutdown: bool,
    requests_rx: mpsc::UnboundedReceiver<(Request, oneshot::Sender<Result<Value, Value>>)>,
    notifications_rx: mpsc::UnboundedReceiver<(Notification, oneshot::Sender<()>)>,
    pending_requests: HashMap<u32, oneshot::Sender<Result<Value, Value>>>,
    pending_notifications: Vec<oneshot::Sender<()>>,
}

impl Endpoint {
    fn handle_msg(&mut self, msg: Message) {
        match msg {
            Message::Request(_) | Message::Notification(_) => {},
            Message::Response(response) => {
                if let Some(response_sender) = self.pending_requests.remove(&response.id) {
                    response_sender.send(response.result).unwrap();
                } 
            }
        }
    }

    fn process_notifications(&mut self) {
        loop {
            match self.notifications_rx.poll().unwrap() {
                Async::Ready(Some((notification, ack_sender))) => {
                    let send_task = self.stream
                        .start_send(Message::Notification(notification))
                        .unwrap();
                    if !send_task.is_ready() {
                        panic!("the sink is full")
                    }
                    self.pending_notifications.push(ack_sender);
                }
                Async::Ready(None) => {
                    self.shutdown = true;
                    return;
                }
                Async::NotReady => return,
            }
        }
    }

    fn process_requests(&mut self) {
        loop {
            match self.requests_rx.poll().unwrap() {
                Async::Ready(Some((mut request, response_sender))) => {
                    self.request_id += 1;
                    request.id = self.request_id;
                    let send_task = self.stream.start_send(Message::Request(request)).unwrap();
                    if !send_task.is_ready() {
                        panic!("the sink is full")
                    }
                    self.pending_requests
                        .insert(self.request_id, response_sender);
                }
                Async::Ready(None) => {
                    self.shutdown = true;
                    return;
                }
                Async::NotReady => return,
            }
        }
    }

    fn flush(&mut self) {
        if self.stream.poll_complete().unwrap().is_ready() {
            for ack_sender in self.pending_notifications.drain(..) {
                ack_sender.send(()).unwrap();
            }
        }
    }
}

impl Future for Endpoint {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.stream.poll().unwrap() {
                Async::Ready(Some(msg)) => self.handle_msg(msg),
                Async::Ready(None) => {
                    return Ok(Async::Ready(()));
                }
                Async::NotReady => break,
            }
        }
        if self.shutdown {
            if self.pending_requests.is_empty() {
                Ok(Async::Ready(()))
            } else {
                Ok(Async::NotReady)
            }
        } else {
            self.process_notifications();
            self.process_requests();
            self.flush();
            Ok(Async::NotReady)
        }
    }
}

/// A future that returns a `Endpoint` when it completes successfully.
pub struct Connection {
    client_rx: oneshot::Receiver<Client>,
    client_chan_cancelled: bool,
    error_rx: oneshot::Receiver<io::Error>,
    error_chan_cancelled: bool,
}

impl Connection {
    fn poll_error(&mut self) -> Option<io::Error> {
        if self.error_chan_cancelled {
            return None;
        }
        match self.error_rx.poll() {
            Ok(Async::Ready(e)) => Some(e),
            Ok(Async::NotReady) => None,
            Err(_) => {
                self.error_chan_cancelled = true;
                None
            }
        }
    }

    fn poll_client(&mut self) -> Option<Client> {
        if self.client_chan_cancelled {
            return None;
        }
        match self.client_rx.poll() {
            Ok(Async::Ready(client)) => Some(client),
            Ok(Async::NotReady) => None,
            Err(_) => {
                self.client_chan_cancelled = true;
                None
            }
        }
    }
}

impl Future for Connection {
    type Item = Client;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(client) = self.poll_client() {
            Ok(Async::Ready(client))
        } else if let Some(e) = self.poll_error() {
            Err(e)
        } else if self.client_chan_cancelled && self.error_chan_cancelled {
            panic!("Failed to receive outcome of the connection");
        } else {
            Ok(Async::NotReady)
        }
    }
}

