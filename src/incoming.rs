use std::io::Result;
use std::io::{Read, Write};
use std::collections::VecDeque;
use std::collections::HashMap;

use mio::*;
use mio::util::*;
use mio::unix::UnixListener;
use bytes::ByteBuf;

use super::outgoing::Outgoing;
use super::connection::Connection;
use super::SyslogForwarder;

pub struct Incoming {
  sockets: HashMap<Token, UnixListener>,
  connections: Slab<Connection>,
  message_queue: VecDeque<ByteBuf>
}

impl Incoming {
  pub fn new(sockets: HashMap<Token, UnixListener>, token_start: usize) -> Incoming {
    Incoming {
      sockets: sockets,
      connections: Slab::new_starting_at(Token(token_start), 128),
      message_queue: VecDeque::new()
    }
  }

  pub fn accept(&mut self, event_loop: &mut EventLoop<SyslogForwarder>, token: Token) -> Result<()> {
    // Accept the connection and unrap the Result to get at the actual result listener
    let socket = self.sockets.get(&token).expect("Incoming socket with token not found").accept().unwrap().unwrap();
    let connection = Connection::new(socket);
    let token = self.connections.insert(connection)
      .ok().expect("could not add connection to slab");

    // Register the connection
    self.connections[token].update_token(token);

    event_loop.register_opt(self.connections[token].socket(),
                            token,
                            EventSet::readable() | EventSet::hup(),
                            PollOpt::level())
      .ok().expect("could not register socket with event loop");

    Ok(())
  }

  pub fn readable(&mut self, outgoing: &Outgoing, event_loop: &mut EventLoop<SyslogForwarder>, token: Token) -> Result<()> {
    let mut buf = ByteBuf::mut_with_capacity(2048);

    match self.connections[token].mut_socket().read(&mut buf) {
      Ok(Some(0)) => {
        // remove the connection which closes the socket which makes epoll stop
        // watching the fd
        self.connections.remove(token);
      }
      Ok(Some(_bytes_read)) => {
        self.message_queue.push_back(buf.flip());
      }
      Ok(None) => {
      }
      Err(e) => {
        println!("INCOMING: error on socket read(): {:?}", e);
      }
    }

    // Add a listener for our outgoing udp socket
    outgoing.reregister(event_loop);

    Ok(())
  }

  #[inline]
  pub fn mut_message_queue(&mut self) -> &mut VecDeque<ByteBuf> {
    &mut self.message_queue
  }

  #[inline]
pub fn message_queue(&self) -> &VecDeque<ByteBuf> {
  &self.message_queue
}
}
