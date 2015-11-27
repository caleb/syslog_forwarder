use mio::*;
use mio::unix::UnixStream;

pub struct Connection {
  socket: UnixStream,
  token: Token
}

impl Connection {
  pub fn new(socket: UnixStream) -> Connection {
    Connection {
      socket: socket,
      token: Token(0)
    }
  }

  #[inline]
  pub fn socket(&self) -> &UnixStream {
    &self.socket
  }

  #[inline]
  pub fn mut_socket(&mut self) -> &mut UnixStream {
    &mut self.socket
  }

  #[inline]
  pub fn update_token(&mut self, token: Token) {
    self.token = token;
  }
}
