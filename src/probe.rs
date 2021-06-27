//! Core node inspection functions.

use tokio::io::{AsyncRead, AsyncWrite};

use tokio_util::codec::Framed;

use blather::Telegram;

use crate::err::Error;
use crate::types;


#[derive(Debug)]
pub struct DDLinkInfo {
  pub engine: String,
  pub protocol: types::node::ddlnk::Protocol,
  pub protimpl: types::node::ddlnk::ProtImpl
}


#[derive(Debug)]
pub struct NodeInfo {
  pub version: String,
  pub os_name: String,
  pub nodetype: types::node::Type,
  pub ddlnk: DDLinkInfo
}


/*
fn parse<T: FromStr>(
  params: &blather::Params,
  field: &str
) -> Result<T, Error> {
  let val = params.get_str("ddmw.node").map_or_else(
    || Err(Error::miss_data(format!("{} not found", field))),
    |s| Ok(s.parse::<T>()?)
  )?;

  Ok(val)
}
*/


pub async fn get_nodeinfo<T: AsyncRead + AsyncWrite + Unpin>(
  conn: &mut Framed<T, blather::Codec>
) -> Result<NodeInfo, Error> {
  let mut tg = Telegram::new();
  tg.set_topic("GetNodeInfo")?;
  let params = crate::sendrecv(conn, &tg).await?;

  let nodetype = params.get_str("ddmw.node").map_or_else(
    || Err(Error::miss_data("ddmw.node not found")),
    |s| s.parse::<types::node::Type>()
  )?;

  let version = params.get_str("ddmw.version").map_or_else(
    || Err(Error::miss_data("ddmw.version not found")),
    |s| Ok(s.to_string())
  )?;

  let os_name = params.get_str("os.name").map_or_else(
    || Err(Error::miss_data("os.name not found")),
    |s| Ok(s.to_string())
  )?;

  let engine = params.get_str("ddmw.ddlnk.engine").map_or_else(
    || Err(Error::miss_data("ddmw.ddlnk.engine not found")),
    |s| Ok(s.to_string())
  )?;

  let protocol = params.get_str("ddmw.ddlink.protocol").map_or_else(
    || Err(Error::miss_data("ddmw.ddlink.protocol not found")),
    |s| s.parse::<types::node::ddlnk::Protocol>()
  )?;

  let protimpl = params.get_str("ddmw.ddlink.protimpl").map_or_else(
    || Err(Error::miss_data("ddmw.ddlink.protimpl not found")),
    |s| s.parse::<types::node::ddlnk::ProtImpl>()
  )?;

  Ok(NodeInfo {
    version,
    os_name,
    nodetype,
    ddlnk: DDLinkInfo {
      engine,
      protocol,
      protimpl
    }
  })
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
