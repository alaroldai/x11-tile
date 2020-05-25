use crate::{
    screen_resources,
    conn
};

use {
    std::{
        rc::Rc,
    },
    xcb::{base as xbase, randr as xrandr, xproto},
    anyhow::{ anyhow, Error },
};

pub struct Window {
    pub cm: Rc<conn::Connection>,
    pub xwin: xproto::Window,
}

impl Window {
    pub fn get_property<T: Clone>(
        &self,
        property: &str,
        type_: xproto::Atom,
        count: usize,
    ) -> Result<Vec<T>, Error> {
        let atom = self.cm.get_atom(property)?;
        Ok(xproto::get_property(
            &self.cm.connection,
            false,
            self.xwin,
            atom,
            type_,
            0,
            (count * std::mem::size_of::<T>()) as u32,
        )
        .get_reply()?
        .value::<T>()
        .iter()
        .cloned()
        .collect::<Vec<T>>())
    }

    pub fn send_event<T>(
        &self,
        propagate: bool,
        mask: u32,
        event: &xcb::Event<T>,
    ) -> Result<(), Error> {
        xproto::send_event(&self.cm.connection, propagate, self.xwin, mask, event)
            .request_check()
            .map_err(|e| anyhow!("{}", e))
    }

    pub fn get_geometry(&self) -> Result<xproto::GetGeometryReply, Error> {
        xproto::get_geometry(&self.cm.connection, self.xwin)
        .get_reply()
        .map_err(|e| anyhow!("Couldn't get window geometry: {}", e))
    }
}

pub struct RootWindow {
    pub inner: Window
}

impl RootWindow {
    pub fn get_screen_resources_current(
        &self,
    ) -> Result<screen_resources::ScreenResources, Error> {
        Ok(screen_resources::ScreenResources {
            cm: self.inner.cm.clone(),
            xsr: xrandr::get_screen_resources_current(&self.inner.cm.connection, self.inner.xwin)
            .get_reply()
            .map_err(|e| anyhow!("Couldn't get screen resources: {}", e))?
        })
    }

    pub fn get_active_window(
        &self,

    ) -> Result<Window, Error> {
        Ok(Window {
            cm: self.inner.cm.clone(),
            xwin: self.inner.get_property("_NET_ACTIVE_WINDOW", xproto::ATOM_WINDOW, 1)?[0],
        })
    }

    pub fn supports(&self, msg: &str) -> Result<bool, Error> {
        let atom = self.inner.cm.get_atom(msg)?;
        let list: Vec<xproto::Atom> = self.get_property(
            "_NET_SUPPORTED",
            xproto::ATOM_ATOM,
            1024,
        )?;
        Ok(list.contains(&atom))
    }

    pub fn get_property<T: Clone>(&self, property: &str, type_: xproto::Atom, count: usize) -> Result<Vec<T>, Error> {
        self.inner.get_property(property, type_, count)
    }

    pub fn send_event<T>(
        &self,
        propagate: bool,
        mask: u32,
        event: &xcb::Event<T>,
    ) -> Result<(), Error> {
        self.inner.send_event(propagate, mask, event)
    }
}