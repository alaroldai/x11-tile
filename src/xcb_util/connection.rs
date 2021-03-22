use xcb::{
  base as xbase,
  xproto,
};

use anyhow::Error;

pub trait ConnectionExt {
  fn get_atom(&self, atom_: &str) -> Result<xproto::Atom, Error>;
}

impl ConnectionExt for xbase::Connection {
  fn get_atom(&self, atom_: &str) -> Result<xproto::Atom, Error> {
    Ok(xproto::intern_atom(&self, true, atom_).get_reply()?.atom())
  }
}
