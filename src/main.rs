#![feature(libc)]
#![feature(collections)]

extern crate nix;
extern crate mio;
extern crate getopts;

#[macro_use]
extern crate lazy_static;

mod connection;
mod incoming;
mod outgoing;

use incoming::Incoming;
use outgoing::Outgoing;

use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Mutex;
use std::collections::HashMap;

use mio::*;
use mio::udp::UdpSocket;
use mio::unix::UnixListener;
use nix::sys::signal;

use getopts::Options;

lazy_static! {
  static ref SOCKET_PATHS:Mutex<Vec<String>> = {
    Mutex::new(Vec::new())
  };
}

fn get_socket_paths() -> Vec<String> {
  SOCKET_PATHS.lock().unwrap().clone()
}

fn set_socket_paths(paths: &[String]) {
  let mut lock = SOCKET_PATHS.lock().unwrap();
  let mut v = Vec::new();
  v.push_all(paths);

  *lock = v;
}

fn unlink_sockets() {
  for path in get_socket_paths() {
    let _ = fs::remove_file(path);
  }
}

extern fn handle_sigint(_:i32) {
  unlink_sockets();
  panic!();
}

fn register_signals() {
  let sig_action = signal::SigAction::new(handle_sigint,
                                          signal::SockFlag::empty(),
                                          signal::SigSet::empty());
  let _ = signal::sigaction(signal::SIGINT, &sig_action);
}

/*
 * Our application structure that holds the client and server structures passed
 * around by Mio
 */
struct SyslogForwarder {
  incoming: Incoming,
  outgoing: Outgoing,
  incoming_token_max: usize
}

impl SyslogForwarder {
  fn new(incoming_sockets:HashMap<Token, NonBlock<UnixListener>>,
         outgoing_socket:NonBlock<UdpSocket>,
         syslog_address:SocketAddr) -> SyslogForwarder {

    let mut max:usize = 0;
    for k in incoming_sockets.keys() {
      if k.as_usize() > max {
        max = k.as_usize();
      }
    }

    SyslogForwarder {
      incoming_token_max: max,
      incoming: Incoming::new(incoming_sockets, max + 1),
      outgoing: Outgoing::new(outgoing_socket, syslog_address)
    }
  }

  #[inline]
  fn is_connection_token(&self, token: Token) -> bool {
    token.as_usize() > self.incoming_token_max
  }

  #[inline]
  fn is_incoming_token(&self, token: Token) -> bool {
    token.as_usize() <= self.incoming_token_max
  }
}

/*
 * Set up the top lever handler that will delegate to other handlers
 */
const OUTGOING: Token = Token(0);

impl Handler for SyslogForwarder {
  type Timeout = ();
  type Message = ();

  fn readable(&mut self, event_loop: &mut EventLoop<SyslogForwarder>, token: Token, _hint: ReadHint) {
    match token {
      OUTGOING => panic!("Got readable for OUTGOING (token 0)"),
      t if self.is_incoming_token(t) => self.incoming.accept(event_loop, token).unwrap(),
      t => self.incoming.readable(&self.outgoing, event_loop, t).unwrap()
    }
  }

  fn writable(&mut self, event_loop: &mut EventLoop<SyslogForwarder>, token: Token) {
    match token {
      OUTGOING => self.outgoing.writable(&mut self.incoming, event_loop).unwrap(),
      _ => panic!("Got writable for non-OUTGOING fd")
    }
  }

  fn notify(&mut self, _event_loop: &mut EventLoop<Self>, _msg: ()) {
    println!("Notify");
  }

  fn timeout(&mut self, _event_loop: &mut EventLoop<Self>, _timeout: ()) {
    println!("Timeout");
  }

  fn interrupted(&mut self, _event_loop: &mut EventLoop<Self>) {
    println!("Interrupted");
  }
}

fn main() {
  // Parse the commend line arguments
  let args:Vec<String> = env::args().collect();
  let _program = args[0].clone();

  let mut opts = Options::new();
  opts.optopt("d", "destination", "set the destination address:port", "ADDRESS");
  opts.optmulti("s", "socket", "set the incoming socket path (defaults to /dev/log)", "PATH");
  let opt_matches = match opts.parse(&args[1..]) {
    Ok(m) => m,
    Err(f) => panic!(f.to_string())
  };

  let mut socket_locations = opt_matches.opt_strs("s");
  let syslog_addr = opt_matches.opt_str("d").expect("You must provide a destination address:port with the -d option").parse().unwrap();

  if socket_locations.len() == 0 {
    socket_locations.push("/dev/log".to_string());
  }

  set_socket_paths(&socket_locations);
  register_signals();
  unlink_sockets();

  let mut event_loop = EventLoop::new().unwrap();

  let mut incoming_map = HashMap::new();
  let mut i = 1;
  // Create a listener that listens on a unix socket
  for socket_path in &socket_locations {
    let addr = Path::new(&socket_path);
    let incoming_socket = unix::bind(addr).unwrap();

    let token = Token(i);
    i += 1;

    event_loop.register(&incoming_socket, token).unwrap();
    incoming_map.insert(token, incoming_socket);
  }

  // Create a socket to talk to the syslog server
  let outgoing_socket = mio::udp::v4().unwrap();
  event_loop.register_opt(&outgoing_socket,
                          OUTGOING,
                          Interest::writable(),
                          PollOpt::oneshot()).unwrap();

  let mut syslog_forwarder = SyslogForwarder::new(incoming_map,
                                                  outgoing_socket,
                                                  syslog_addr);
  event_loop.run(&mut syslog_forwarder).unwrap();
}
