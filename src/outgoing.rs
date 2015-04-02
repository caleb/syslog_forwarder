use std::io::Result;
use std::net::SocketAddr;

use mio::*;
use mio::udp::UdpSocket;

use super::OUTGOING;
use super::incoming::Incoming;
use super::SyslogForwarder;

pub struct Outgoing {
  pub socket: NonBlock<UdpSocket>,
  pub addr: SocketAddr
}

impl Outgoing {
  pub fn new(socket: NonBlock<UdpSocket>, addr: SocketAddr) -> Outgoing {
    Outgoing {
      socket: socket,
      addr: addr
    }
  }

  pub fn writable(&mut self, incoming: &mut Incoming, _event_loop: &mut EventLoop<SyslogForwarder>) -> Result<()> {
    if incoming.message_queue().len() > 0 {
      for mut buf in incoming.mut_message_queue().drain() {
        let _result = self.socket.send_to(&mut buf, &self.addr);
      }
    }

    Ok(())
  }

  pub fn reregister(&self, event_loop: &mut EventLoop<SyslogForwarder>) {
    event_loop.reregister(&self.socket,
                          OUTGOING,
                          Interest::writable(),
                          PollOpt::oneshot()).ok();
  }
}
