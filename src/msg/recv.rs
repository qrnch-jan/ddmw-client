//! Functions for receiving messages.

use std::future::Future;
use std::path::PathBuf;

use tokio::io::{AsyncRead, AsyncWrite};

use tokio_stream::StreamExt;

use tokio_util::codec::Framed;

use bytes::{Bytes, BytesMut};

use blather::{codec, KVLines, Params, Telegram};

use crate::err::Error;


pub enum SubCh {
  Num(u8),
  Name(String)
}

pub struct SubInfo {
  pub ch: SubCh
}


/// Subscribe to an application message channel.
pub async fn subscribe<C>(
  conn: &mut Framed<C, blather::Codec>,
  subinfo: SubInfo
) -> Result<(), Error>
where
  C: AsyncRead + AsyncWrite + Unpin
{
  let mut tg = Telegram::new();
  tg.set_topic("Sub")?;
  match subinfo.ch {
    SubCh::Num(ch) => {
      tg.add_param("Ch", ch)?;
    }
    SubCh::Name(nm) => {
      tg.add_param("Ch", nm)?;
    }
  }
  crate::sendrecv(conn, &tg).await?;

  Ok(())
}


/// Storage type used to request how a messsage's metadata and/or payload
/// should be stored/parsed.
pub enum StoreType {
  /// Don't store
  None,

  /// Store it as a [`bytes::Bytes`]
  Bytes,

  /// Store it as a [`bytes::BytesMut`]
  BytesMut,

  /// Parse and store it in a [`blather::Params`] buffer
  Params,

  /// Parse and store it as a [`blather::KVLines`] buffer
  KVLines,

  /// Store it in a file
  File(PathBuf)
}


/// Storage buffer for metadata and payload.
pub enum Storage {
  /// Return data as a [`bytes::Bytes`] buffer.
  Bytes(Bytes),

  /// Return data as a [`bytes::BytesMut`] buffer.
  BytesMut(BytesMut),

  /// Return data as a parsed [`Params`] buffer.
  Params(Params),

  /// Return data as a parsed [`KVLines`] buffer.
  KVLines(KVLines),

  /// A file whose location was requested by the application.
  File(PathBuf),

  /// A file whose location was specified by DDMW.  It is the responsibility
  /// of the application to move the file from its current location to an
  /// application specific storage, or delete the file if it is not relevant.
  LocalFile(PathBuf)
}


/// Representation of a received message with its optional metadata and
/// payload.
pub struct Msg {
  pub cmd: u32,
  pub meta: Option<Storage>,
  pub payload: Option<Storage>
}


/// Message information buffer passed to storate request callback closures.
pub struct MsgInfo {
  /// Message command number.
  pub cmd: u32,

  /// Length of message metadata.
  pub metalen: u32,

  /// Length of message payload.
  pub payloadlen: u64
}


/// Receive a single message.
///
/// The `storeq` is a closure that, if there's metadata and/or payload, can be
/// called to allow the application to request how the metadata and payload
/// will be stored.  It's only argument is a reference to a [`Params`] buffer
/// which was extracted from the incoming `Msg` `Telegram`.
///
/// It is up to the closure to return a tuple of two `StorageType` enum values,
/// where the first one denotes the requested storage type for the metadata,
/// and the second one denotes the requested storage type for the message
/// payload.
///
/// # Notes
/// - The `storeq` closure is not called if there's neither metadata nor
///   payload associated with the incoming message.
/// - `storeq` is referred to as a _request_ because the function does not
///   necessarily need to respect the exact choice.  Specifically, there are
///   two special cases:
///   - If the size of metadata or payload (but not both) is zero, then its
///     respective member in [`Msg`] will be None.  (For instance, specifying a
///     file will not yield an empty file).
///   - If the received content was stored as a local file that was stored by
///     the DDMW server, then the library will always return a
///     [`Storage::LocalFile`] for this content.
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use tokio::net::TcpStream;
/// use tokio_util::codec::Framed;
/// use ddmw_client::{
///   conn,
///   msg::{
///     self,
///     recv::{StoreType, Msg}
///   }
/// };
///
/// // Enter an loop which keeps receiving messages until the connection is
/// // dropped.
/// async fn get_message(conn: &mut Framed<TcpStream, blather::Codec>) -> Msg {
///   msg::recv(
///     conn,
///     |_mi| {
///       // A new message is arriving; give the application the opportunity to
///       // choose how to store the message metadata and payload.
///       // Request file storage.
///       // Note:  This request may be overridden.
///       let metafile = PathBuf::from("msg.meta");
///       let payloadfile = PathBuf::from("msg.payload");
///       Ok((StoreType::File(metafile), StoreType::File(payloadfile)))
///     },
///   ).await.unwrap()
/// }
/// ```
pub async fn recv<C, S>(
  conn: &mut Framed<C, blather::Codec>,
  storeq: S
) -> Result<Msg, Error>
where
  C: AsyncRead + AsyncWrite + Unpin,
  S: FnMut(&MsgInfo) -> Result<(StoreType, StoreType), Error>
{
  // Wait for the next frame, and exiect it to be a Telegram.
  if let Some(o) = conn.next().await {
    let o = o?;
    match o {
      codec::Input::Telegram(tg) => {
        // Got the expetected Telegram -- make sure that it's has a "Msg"
        // topic.
        if let Some(topic) = tg.get_topic() {
          if topic == "Msg" {
            // Convert to a Params buffer, since we no longer need the topic
            let mp = tg.into_params();

            return proc_inbound_msg(conn, mp, storeq).await;
          } else if topic == "Fail" {
            return Err(Error::ServerError(tg.into_params()));
          }
        }
      }
      _ => {
        return Err(Error::BadState(
          "Unexpected codec input type.".to_string()
        ));
      }
    }
    return Err(Error::bad_state("Unexpected reply from server."));
  }

  Err(Error::Disconnected)
}


/// Enter a loop which will keep receiving messages until connection is closed
/// or a killswitch is triggered.
///
/// Returns `Ok()` if the loop was terminated by the killswitch.
///
/// # Example
/// The following example illustrates how to write a function that will keep
/// receiving messages.
///
/// ```no_run
/// use std::path::PathBuf;
/// use tokio::net::TcpStream;
/// use tokio_util::codec::Framed;
/// use ddmw_client::{
///   conn,
///   msg::{self, recv::StoreType}
/// };
///
/// // Enter an loop which keeps receiving messages until the connection is
/// // dropped.
/// async fn get_messages(conn: &mut Framed<TcpStream, blather::Codec>) {
///   let mut idx = 0;
///
///   msg::recvloop(
///     conn,
///     None,
///     |mi| {
///       // A new message is arriving; give the application the opportunity to
///       // choose how to store the message metadata and payload.
///
///       // Choose what to do with message metadata
///       let meta_store = if mi.metalen > 1024*1024 {
///         // Too big
///         StoreType::None
///       } else {
///         // Store it in a memory buffer
///         StoreType::Bytes
///       };
///
///       // Choose what to do with message payload
///       let payload_store = if mi.payloadlen > 16*1024*1024 {
///         // Bigger than 16MB; too big, just ignore it
///         StoreType::None
///       } else if mi.payloadlen > 256*1024 {
///         // It's bigger than 256K -- store it in a file
///         let payloadfile = format!("{:x}.payload", idx);
///         idx += 1;
///         StoreType::File(PathBuf::from(payloadfile))
///       } else {
///         // It's small enough to store in a memory buffer
///         StoreType::Bytes
///       };
///
///       Ok((meta_store, payload_store))
///     },
///     |msg| {
///       // Process message
///       Ok(())
///     }
///   ).await.unwrap();
/// }
/// ```
// ToDo: yield, when it becomes available
pub async fn recvloop<C, S, P>(
  conn: &mut Framed<C, blather::Codec>,
  kill: Option<killswitch::Shutdown>,
  mut storeq: S,
  procmsg: P
) -> Result<(), Error>
where
  C: AsyncRead + AsyncWrite + Unpin,
  S: FnMut(&MsgInfo) -> Result<(StoreType, StoreType), Error>,
  P: Fn(Msg) -> Result<(), Error>
{
  if let Some(kill) = kill {
    loop {
      tokio::select! {
        msg = recv(conn, &mut storeq) => {
          let msg = msg?;
          procmsg(msg)?;
        }
        _ = kill.wait() => {
          // An external termination request was received, so break out of loop
          break;
        }
      }
    }
  } else {
    // No killswitch supplied -- just keep running until disconnection
    loop {
      let msg = recv(conn, &mut storeq).await?;
      procmsg(msg)?;
    }
  }

  Ok(())
}


/// This is the same as [`recvloop()`], but it assumes the message processing
/// closure returns a [`Future`].
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use tokio::net::TcpStream;
/// use tokio_util::codec::Framed;
/// use ddmw_client::{
///   conn,
///   msg::{self, recv::StoreType}
/// };
///
/// // Enter an loop which keeps receiving messages until the connection is
/// // dropped.
/// async fn get_messages(conn: &mut Framed<TcpStream, blather::Codec>) {
///   let mut idx = 0;
///
///   msg::recvloop_a(
///     conn,
///     None,
///     |mi| {
///       Ok((StoreType::Bytes, StoreType::Bytes))
///     },
///     |msg| {
///       async {
///         // Process message
///         Ok(())
///       }
///     }
///   ).await.unwrap();
/// }
/// ```
pub async fn recvloop_a<C, S, F, P>(
  conn: &mut Framed<C, blather::Codec>,
  kill: Option<killswitch::Shutdown>,
  mut storeq: S,
  procmsg: P
) -> Result<(), Error>
where
  C: AsyncRead + AsyncWrite + Unpin,
  S: FnMut(&MsgInfo) -> Result<(StoreType, StoreType), Error>,
  F: Future<Output = Result<(), Error>>,
  P: Fn(Msg) -> F
{
  if let Some(kill) = kill {
    loop {
      tokio::select! {
        msg = recv(conn, &mut storeq) => {
          let msg = msg?;
          procmsg(msg).await?;
        }
        _ = kill.wait() => {
          // An external termination request was received, so break out of loop
          break;
        }
      }
    }
  } else {
    // No killswitch supplied -- just keep running until disconnection
    loop {
      let msg = recv(conn, &mut storeq).await?;
      procmsg(msg).await?;
    }
  }

  Ok(())
}


async fn proc_inbound_msg<C, S>(
  conn: &mut Framed<C, blather::Codec>,
  mp: Params,
  mut storeq: S
) -> Result<Msg, Error>
where
  C: AsyncRead + AsyncWrite + Unpin,
  S: FnMut(&MsgInfo) -> Result<(StoreType, StoreType), Error>
{
  let metalen = if mp.have("MetaLen") {
    mp.get_param::<u32>("MetaLen")?
  } else {
    0u32
  };

  let payloadlen = if mp.have("Len") {
    mp.get_param::<u64>("Len")?
  } else {
    0u64
  };

  // Parse command
  let cmd = if mp.have("Cmd") {
    mp.get_param::<u32>("Cmd")?
  } else {
    0
  };

  // ToDo: Parse mp and check if metadata and/or payload is passed from the
  //       server using a local file path.  If it is, then return it to the
  //       application using Storage::LocalFile(PathBuf).

  // If the Params contains either a Len or a MetaLen keyword, then
  // call the application callback to determine how it wants the data
  // stored.
  // ToDo: - If metadata and payload are stored as "local files", then don't
  //         call application; force to Storage::LocalFile
  let (meta_store, payload_store) = if metalen != 0 || payloadlen != 0 {
    // Call the application callback, passing a few Msg parameters, to ask it
    // in what form it would like the metadata and payload.

    let mi = MsgInfo {
      cmd,
      metalen,
      payloadlen
    };

    let (ms, ps) = storeq(&mi)?;
    let ms = if metalen != 0 { Some(ms) } else { None };
    let ps = if metalen != 0 { Some(ps) } else { None };
    (ms, ps)
  } else {
    (None, None)
  };

  //
  // At this point, if meta_store is None, it means there was no message
  // metadata, and we'll skip this altogether and return None to the
  // application for the metadata.
  //
  // If meta_store is Some, then request the appropriate type from the
  // blather's Codec.
  //
  let meta = if let Some(meta_store) = meta_store {
    match meta_store {
      StoreType::None => {
        // This happens if the MetaLen is non-zero, but the callback says it
        // doesn't want the data.
        conn.codec_mut().skip(metalen as usize)?;
      }
      StoreType::Bytes => {
        conn.codec_mut().expect_bytes(metalen as usize)?;
      }
      StoreType::BytesMut => {
        conn.codec_mut().expect_bytesmut(metalen as usize)?;
      }
      StoreType::Params => {
        conn.codec_mut().expect_params();
      }
      StoreType::KVLines => {
        conn.codec_mut().expect_kvlines();
      }
      StoreType::File(ref fname) => {
        conn.codec_mut().expect_file(fname, metalen as usize)?;
      }
    }

    get_content(conn).await?
  } else {
    None
  };


  let payload = if let Some(payload_store) = payload_store {
    match payload_store {
      StoreType::None => {
        // This happens if the Len is non-zero, but the callback says it
        // doesn't want the data.
        conn.codec_mut().skip(payloadlen as usize)?;
      }
      StoreType::Bytes => {
        conn.codec_mut().expect_bytes(payloadlen as usize)?;
      }
      StoreType::BytesMut => {
        conn.codec_mut().expect_bytesmut(payloadlen as usize)?;
      }
      StoreType::Params => {
        conn.codec_mut().expect_params();
      }
      StoreType::KVLines => {
        conn.codec_mut().expect_kvlines();
      }
      StoreType::File(ref fname) => {
        conn.codec_mut().expect_file(fname, payloadlen as usize)?;
      }
    }

    get_content(conn).await?
  } else {
    None
  };


  Ok(Msg { cmd, meta, payload })
}


/// Translate an incoming frame from the [`blather::Codec`] into a [`Storage`]
/// type.
async fn get_content<C>(
  conn: &mut Framed<C, blather::Codec>
) -> Result<Option<Storage>, Error>
where
  C: AsyncRead + AsyncWrite + Unpin
{
  if let Some(o) = conn.next().await {
    let o = o?;
    match o {
      codec::Input::SkipDone => Ok(None),
      codec::Input::Bytes(bytes) => Ok(Some(Storage::Bytes(bytes))),
      codec::Input::BytesMut(bytes) => Ok(Some(Storage::BytesMut(bytes))),
      codec::Input::Params(params) => Ok(Some(Storage::Params(params))),
      codec::Input::KVLines(kvlines) => Ok(Some(Storage::KVLines(kvlines))),
      codec::Input::File(fname) => Ok(Some(Storage::File(fname))),
      _ => Err(Error::bad_state("Unexpected codec input type."))
    }
  } else {
    Err(Error::Disconnected)
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
