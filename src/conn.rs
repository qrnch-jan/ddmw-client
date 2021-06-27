//! Methods used to establish connections to DDMW Core servers' client
//! interfaces.

use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

#[cfg(unix)]
use std::path::{Path, PathBuf};

use futures::sink::SinkExt;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

#[cfg(unix)]
use tokio::net::UnixStream;

use tokio_stream::StreamExt;

use tokio_util::codec::Framed;

#[cfg(unix)]
use tokio_util::either::Either;

use blather::{codec, Telegram};

use crate::auth::Auth;

use crate::err::Error;


/// Protocol selection enum.
pub enum ProtAddr {
  /// Connect over TCP/IP.  The `String` is a socket address in the form
  /// `<host>:<port>`.
  Tcp(String),

  /// Connect over unix local domain sockets.  The `PathBuf` is a file system
  /// socket path.
  #[cfg(unix)]
  Uds(PathBuf)
}

impl FromStr for ProtAddr {
  type Err = Error;

  /// Parse a `&str` and turn it into a `ProtAddr`.
  ///
  /// On unixy platforms if the `addr` contains one or more slashes (`/`) it is
  /// assumed the address is a unix local domain socket address.  Otherwise
  /// it is assumed the address is an IP socket address, in the form
  /// `<host>:<port>`.
  fn from_str(addr: &str) -> Result<Self, Self::Err> {
    #[cfg(unix)]
    if let Some(_) = addr.find("/") {
      // Assume local domain socket
      Ok(ProtAddr::Uds(PathBuf::from(addr)))
    } else {
      // Assume IP socket address
      Ok(ProtAddr::Tcp(addr.to_string()))
    }

    #[cfg(windows)]
    Ok(ProtAddr::Tcp(addr.to_string()))
  }
}

impl fmt::Display for ProtAddr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      #[cfg(unix)]
      ProtAddr::Uds(sa) => {
        // ToDo: Return error if it's not really a valid Unicode string.
        write!(f, "{}", sa.display())
      }
      ProtAddr::Tcp(sa) => {
        write!(f, "{}", sa)
      }
    }
  }
}


/// Framed type alias for Unix platforms where a connection can be either
/// TcpStream or UnixStream.
#[cfg(unix)]
pub type Frm = Framed<Either<TcpStream, UnixStream>, blather::Codec>;

/// Framed type alias for Windows.  This exists merely due to UnixStream not
/// currently being available on Windows.  Once it is, this should be removed.
#[cfg(windows)]
pub type Frm = Framed<TcpStream, blather::Codec>;


/// Connect to one of the DDMW core's client interfaces and optionally attempt
/// to authenticate.
///
/// If `ProtAddr::Tcp()` is passed into the `pa` argument an TCP/IP connection
/// will be attempted.  If `ProtAddr::Uds()` (currently only available on
/// unix-like platforms) is used a unix local domain socket connection will be
/// attempted.
///
/// If `auth` has `Some` value, an authentication will be attempted after
/// successful connection.  If the authentication fails the entire connection
/// will fail.  To be able to keep the connection up in case the authentication
/// fails, pass `None` to the `auth` argument and manually authenticate in the
/// application.
pub async fn connect<P>(pa: P, auth: Option<&Auth>) -> Result<Frm, Error>
where
  P: Borrow<ProtAddr>
{
  let mut framed = match pa.borrow() {
    ProtAddr::Tcp(sa) => connect_tcp(sa).await?,

    #[cfg(unix)]
    ProtAddr::Uds(sa) => connect_uds(sa).await?
  };

  if let Some(auth) = auth {
    auth.authenticate(&mut framed).await?;
  }

  Ok(framed)
}


/// Attempt to establish a TCP/IP socket connection.
async fn connect_tcp(addr: &str) -> Result<Frm, Error> {
  let stream = TcpStream::connect(addr).await?;

  #[cfg(unix)]
  return Ok(Framed::new(Either::Left(stream), blather::Codec::new()));

  #[cfg(windows)]
  return Ok(Framed::new(stream, blather::Codec::new()));
}


/// Attempt to establish a unix domain socket connection.
/// Currently only available on unix-like platforms.
#[cfg(unix)]
async fn connect_uds(addr: &Path) -> Result<Frm, Error> {
  let addr = match addr.to_str() {
    Some(a) => a.to_string(),
    None => unreachable!()
  };
  let stream = UnixStream::connect(addr).await?;
  Ok(Framed::new(Either::Right(stream), blather::Codec::new()))
}


/// Send a telegram then wait for and return the server's reply.
/// If the server returns a `Fail`, it will be returned as
/// `Err(Error::ServerError)`.
pub async fn sendrecv<T: AsyncRead + AsyncWrite + Unpin>(
  conn: &mut Framed<T, blather::Codec>,
  tg: &Telegram
) -> Result<blather::Params, Error> {
  conn.send(tg).await?;
  expect_okfail(conn).await
}


/// Waits for a message and ensures that it's Ok or Fail.
/// Converts Fail state to an Error::ServerError.
/// Returns a Params buffer containig the Ok parameters on success.
pub async fn expect_okfail<T: AsyncRead + AsyncWrite + Unpin>(
  conn: &mut Framed<T, blather::Codec>
) -> Result<blather::Params, Error> {
  if let Some(o) = conn.next().await {
    let o = o?;
    match o {
      codec::Input::Telegram(tg) => {
        if let Some(topic) = tg.get_topic() {
          if topic == "Ok" {
            return Ok(tg.into_params());
          } else if topic == "Fail" {
            return Err(Error::ServerError(tg.into_params()));
          }
        }
      }
      _ => {
        println!("unexpected reply");
      }
    }
    return Err(Error::BadState("Unexpected reply from server.".to_string()));
  }

  Err(Error::Disconnected)
}


#[derive(Debug)]
pub struct WhoAmI {
  pub id: i64,
  pub name: String
}

/// Return the current owner of a connection.
pub async fn whoami<T: AsyncRead + AsyncWrite + Unpin>(
  conn: &mut Framed<T, blather::Codec>
) -> Result<WhoAmI, Error> {
  let tg = Telegram::new_topic("WhoAmI")?;
  let params = sendrecv(conn, &tg).await?;
  let id = params.get_param::<i64>("Id")?;
  let name = params.get_param("Name")?;
  Ok(WhoAmI { id, name })
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
