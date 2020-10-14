use {
  anyhow::{anyhow, Error},
  bitflags::bitflags,
  xcb::{base as xbase, randr as xrandr, xproto},
};

use crate::xcb_util::{connection::ConnectionExt, geometry::*};

bitflags! {
    struct WMSizeHintsFlag: u32 {
        const NONE = 0;
        const US_POSITION   = 1 << 0;
        const US_SIZE       = 1 << 1;
        const P_POSITION    = 1 << 2;
        const P_SIZE        = 1 << 3;
        const P_MIN_SIZE    = 1 << 4;
        const P_MAX_SIZE    = 1 << 5;
        const P_RESIZE_INC  = 1 << 6;
        const P_ASPECT      = 1 << 7;
        const BASE_SIZE     = 1 << 8;
        const P_WIN_GRAVITY = 1 << 9;
    }
}

pub trait WindowExt {
  fn get_property<T: Clone>(
    &self,
    connection: &xbase::Connection,
    property: &str,
    type_: xproto::Atom,
    count: usize,
  ) -> Result<Vec<T>, Error>;

  fn change_property<T>(
    &self,
    connection: &xbase::Connection,
    mode: u8,
    property: &str,
    type_: xproto::Atom,
    format: u8,
    data: &[T],
  ) -> Result<(), Error>;

  fn send_event<T>(
    &self,
    connection: &xbase::Connection,
    propagate: bool,
    mask: u32,
    event: &xcb::Event<T>,
  ) -> Result<(), Error>;

  fn get_screen_resources_current(
    &self,
    connection: &xbase::Connection,
  ) -> Result<xrandr::GetScreenResourcesCurrentReply, Error>;

  fn get_active_window(&self, connection: &xbase::Connection) -> Result<xproto::Window, Error>;

  fn get_geometry(&self, connection: &xbase::Connection)
    -> Result<xproto::GetGeometryReply, Error>;

  fn supports(&self, connection: &xbase::Connection, msg: &str) -> Result<bool, Error>;

  fn move_resize(
    &self,
    connection: &xbase::Connection,
    target: xproto::Window,
    new_rect: ScreenRect,
  ) -> Result<(), Error>;
}

impl WindowExt for xproto::Window {
  fn get_property<T: Clone>(
    &self,
    connection: &xbase::Connection,
    property: &str,
    type_: xproto::Atom,
    count: usize,
  ) -> Result<Vec<T>, Error> {
    let atom = connection.get_atom(property)?;
    Ok(
      xproto::get_property(
        connection,
        false,
        *self,
        atom,
        type_,
        0,
        (count * std::mem::size_of::<T>()) as u32,
      )
      .get_reply()?
      .value::<T>()
      .to_vec(),
    )
  }

  fn change_property<T>(
    &self,
    connection: &xbase::Connection,
    mode: u8,
    property: &str,
    type_: xproto::Atom,
    format: u8,
    data: &[T],
  ) -> Result<(), Error> {
    let atom = connection.get_atom(property)?;
    xproto::change_property(connection, mode, *self, atom, type_, format, data);

    Ok(())
  }

  fn send_event<T>(
    &self,
    connection: &xbase::Connection,
    propagate: bool,
    mask: u32,
    event: &xcb::Event<T>,
  ) -> Result<(), Error> {
    xproto::send_event(&connection, propagate, *self, mask, event)
      .request_check()
      .map_err(|e| anyhow!("{}", e))
  }

  fn get_screen_resources_current(
    &self,
    connection: &xbase::Connection,
  ) -> Result<xrandr::GetScreenResourcesCurrentReply, Error> {
    Ok(
      xrandr::get_screen_resources_current(&connection, *self)
        .get_reply()
        .map_err(|e| anyhow!("Couldn't get screen resources: {}", e))?,
    )
  }

  fn get_active_window(&self, connection: &xbase::Connection) -> Result<xproto::Window, Error> {
    Ok(self.get_property(connection, "_NET_ACTIVE_WINDOW", xproto::ATOM_WINDOW, 1)?[0])
  }

  fn get_geometry(
    &self,
    connection: &xbase::Connection,
  ) -> Result<xproto::GetGeometryReply, Error> {
    xproto::get_geometry(connection, *self)
      .get_reply()
      .map_err(|e| anyhow!("Couldn't get window geometry: {}", e))
  }

  fn supports(&self, connection: &xbase::Connection, msg: &str) -> Result<bool, Error> {
    let atom = connection.get_atom(msg)?;
    let list: Vec<xproto::Atom> =
      self.get_property(connection, "_NET_SUPPORTED", xproto::ATOM_ATOM, 1024)?;
    Ok(list.contains(&atom))
  }

  fn move_resize(
    &self,
    connection: &xbase::Connection,
    target: xproto::Window,
    new_rect: ScreenRect,
  ) -> Result<(), Error> {
    if !self.supports(connection, "_NET_MOVERESIZE_WINDOW")? {
      return Err(anyhow!("WM doesn't support _NET_MOVERESIZE_WINDOW"));
    }

    // TODO: KWin's built-in window tiling seems to prevent this from working. Find out why.
    // use xprop to examine window properties

    println!("running move_resize");

    // bits 8-11 are presence bits for x/y/w/h
    // bits 12-15 indicate request source (bit 13 indicates a user-interactive source)
    let flags = xproto::GRAVITY_STATIC | 1 << 8 | 1 << 9 | 1 << 10 | 1 << 11 | 1 << 12;

    let ev = xcb::ClientMessageEvent::new(
      32,
      target,
      connection.get_atom("_NET_MOVERESIZE_WINDOW")?,
      xproto::ClientMessageData::from_data32([
        flags,
        new_rect.origin.x as u32,
        new_rect.origin.y as u32,
        new_rect.size.width as u32,
        new_rect.size.height as u32,
      ]),
    );

    xproto::send_event(
      connection,
      true,
      *self,
      xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY | xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT,
      &ev,
    )
    .request_check()
    .map_err(|_| anyhow!("???"))
  }
}
