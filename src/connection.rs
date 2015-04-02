use mio::*;
use mio::unix::UnixStream;

pub struct Connection {
  socket: NonBlock<UnixStream>,
  token: Token
}

impl Connection {
  pub fn new(socket: NonBlock<UnixStream>) -> Connection {
    Connection {
      socket: socket,
      token: Token(-1)
    }
  }

  #[inline]
  pub fn socket(&self) -> &NonBlock<UnixStream> {
    &self.socket
  }

  #[inline]
  pub fn mut_socket(&mut self) -> &mut NonBlock<UnixStream> {
    &mut self.socket
  }

  #[inline]
  pub fn update_token(&mut self, token: Token) {
    self.token = token;
  }
}
