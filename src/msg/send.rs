//! Functions for sending messages.

use std::borrow::Borrow;
use std::fs;
use std::path::PathBuf;

use tokio::io::{AsyncRead, AsyncWrite};

use tokio_util::codec::Framed;

use futures::sink::SinkExt;

use bytes::Bytes;

use blather::{Params, Telegram};

use crate::auth::Auth;
use crate::conn::{self, ProtAddr};
use crate::types::AppChannel;

use crate::err::Error;


pub enum InputType {
  Params(Params),
  File(PathBuf),
  VecBuf(Vec<u8>),
  Bytes(Bytes)
}

impl InputType {
  fn get_size(&self) -> Result<usize, Error> {
    match self {
      InputType::Params(params) => Ok(params.calc_buf_size()),
      InputType::File(f) => {
        let metadata = fs::metadata(&f)?;
        Ok(metadata.len() as usize)
      }
      InputType::VecBuf(v) => Ok(v.len()),
      InputType::Bytes(b) => Ok(b.len())
    }
  }
}


pub struct Transport {
  pub ch: AppChannel
}

pub struct MsgInfo {
  pub cmd: u32,
  pub meta: Option<InputType>,
  pub payload: Option<InputType>
}


impl MsgInfo {
  fn get_meta_size(&self) -> Result<u32, Error> {
    let sz = match &self.meta {
      Some(meta) => meta.get_size()?,
      None => 0
    };

    if sz > u32::MAX as usize {
      // ToDo: Return out of bounds error
    }

    Ok(sz as u32)
  }


  fn get_payload_size(&self) -> Result<u64, Error> {
    let sz = match &self.payload {
      Some(payload) => payload.get_size()?,
      None => 0
    };

    Ok(sz as u64)
  }
}


/// Connect, optionally authenticate, send message and disconnect.
///
/// This is a convenience function for application that don't need to keep a
/// connection open, and only needs to send a message occasionally.
pub async fn connsend<P, X, M>(
  pa: P,
  auth: Option<&Auth>,
  xfer: X,
  mi: M
) -> Result<String, Error>
where
  P: Borrow<ProtAddr>,
  X: Borrow<Transport>,
  M: Borrow<MsgInfo>
{
  let mut conn = conn::connect(pa, auth).await?;

  send(&mut conn, xfer, mi).await
}


/// Send a message, including (if applicable) its metadata and payload.
///
/// On successful completion returns the transfer identifier.
pub async fn send<T, X, M>(
  conn: &mut Framed<T, blather::Codec>,
  xfer: X,
  mi: M
) -> Result<String, Error>
where
  T: AsyncRead + AsyncWrite + Unpin,
  X: Borrow<Transport>,
  M: Borrow<MsgInfo>
{
  let xfer = xfer.borrow();
  let mi = mi.borrow();

  //
  // Determine length of metadata and payload
  //
  let metalen = mi.get_meta_size()?;
  let payloadlen = mi.get_payload_size()?;

  //
  // Prepare the Msg telegram
  //
  let mut tg = Telegram::new_topic("Msg")?;
  tg.add_param("_Ch", xfer.ch.to_string())?;
  if mi.cmd != 0 {
    tg.add_param("Cmd", mi.cmd)?;
  }
  if metalen != 0 {
    tg.add_param("MetaLen", metalen)?;
  }
  if payloadlen != 0 {
    tg.add_param("Len", payloadlen)?;
  }

  //
  // Request the message transfer
  //
  let params = crate::sendrecv(conn, &tg).await?;

  //
  // Extract the transfer identifier assigned to this message
  //
  let xferid = match params.get_str("XferId") {
    Some(xferid) => xferid.to_string(),
    None => {
      let e = "Missing expected transfer identifier from server reply";
      return Err(Error::MissingData(String::from(e)));
    }
  };

  //
  // Transmit metadata, if applicable, and wait for the server to ACK it
  //
  if let Some(meta) = &mi.meta {
    send_content(conn, meta).await?;
    crate::expect_okfail(conn).await?;
  }

  //
  // Transmit payload, if applicable, and wait for the server to ACK it
  //
  if let Some(payload) = &mi.payload {
    send_content(conn, payload).await?;
    crate::expect_okfail(conn).await?;
  }

  Ok(xferid)
}


async fn send_content<T>(
  conn: &mut Framed<T, blather::Codec>,
  data: &InputType
) -> Result<(), Error>
where
  T: AsyncRead + AsyncWrite + Unpin
{
  match data {
    InputType::Params(params) => Ok(conn.send(params).await?),
    InputType::File(fname) => {
      let mut f = tokio::fs::File::open(fname).await?;
      let _ = tokio::io::copy(&mut f, conn.get_mut()).await?;
      Ok(())
    }
    InputType::VecBuf(v) => Ok(conn.send(v.as_slice()).await?),
    InputType::Bytes(b) => Ok(conn.send(b.as_ref()).await?)
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
