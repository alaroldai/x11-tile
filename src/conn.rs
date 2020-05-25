use {
    anyhow::{anyhow, Error},
    log::trace,
    xcb::{base as xbase, randr as xrandr, xproto},
    euclid::*,
    std::{
        rc::Rc,
    },
};

use crate::{
    window,
    geometry,
    output,
    screen_resources,
    screen,
};

pub struct Connection {
    pub connection: xbase::Connection,
    screen_num: i32,
}

impl Connection {
    pub fn new() -> Result<Rc<Self>, Error> {
        let (connection, screen_num) = xbase::Connection::connect(None)?;
        Ok(Rc::new(Connection {
            connection,
            screen_num,
        }))
    }

    pub fn setup<'a>(self: &'a Rc<Self>) -> xproto::Setup<'a> {
        self.connection.get_setup()
    }

    pub fn root_screen<'a>(self: &'a Rc<Self>) -> Result<screen::Screen<'a>, Error> {
        self.setup()
            .roots()
            .nth(self.screen_num as usize)
            .ok_or_else(|| anyhow!("Couldn't get the default root screen"))
            .map(|scr| {
                screen::Screen {
                    cm: self.clone(),
                    xscreen: scr,
                }
            })
    }

    pub fn root_window(self: &Rc<Self>) -> Result<window::RootWindow, Error> {
        Ok(window::RootWindow {
            inner: window::Window {
                cm: self.clone(),
                xwin: self.root_screen()?.xscreen.root(),
            }
        })
    }

    pub fn get_atom(&self, atom_: &str) -> Result<xproto::Atom, Error> {
        Ok(xproto::intern_atom(&self.connection, true, atom_)
            .get_reply()?
            .atom())
    }

    pub fn flush(&self) {
        self.connection.flush();
    }

    pub fn get_crtc_info(
        &self,
        crtc: xrandr::Crtc,
        timestamp: xproto::Timestamp,
    ) -> Result<xrandr::GetCrtcInfoReply, Error> {
        xrandr::get_crtc_info(&self.connection, crtc, timestamp)
            .get_reply()
            .map_err(|e| anyhow!("Couldn't get CRTC info: {}", e))
    }

    pub fn get_output_id_for_window(
        self: Rc<Self>,
        window: xproto::Window,
    ) -> Result<output::Output, Error> {
        // pretty basic - identifies the output containing the midpoint of the window
        let geom = xproto::get_geometry(&self.connection, window)
            .get_reply()
            .map_err(|e| anyhow!("Couldn't get window geometry: {}", e))?;

        let frame = geometry::ScreenBox::new(
            geometry::XPoint::new(
                geom.x() as i32,
                geom.y() as i32,
            ),
            geometry::XPoint::new(
                geom.x() as i32 + geom.width() as i32,
                geom.x() as i32 + geom.height() as i32,
            ),
        );

        let root_window: window::RootWindow = self.root_window()?;

        let resources: screen_resources::ScreenResources = root_window.get_screen_resources_current()?;

        let mut result = None;
        for output in resources.get_outputs() {
            let area = match output.get_frame(resources.xsr.config_timestamp()).as_ref() {
                Ok(frame) => frame,
                Err(e) => continue,
            }
            .area();

            result = Some(match result {
                None => (output, area),
                Some((existing_output, existing_area)) => {
                    if existing_area > existing_area {
                        (output, area)
                    } else {
                        (existing_output, existing_area)
                    }
                }
            });
        }


        
        result
            .map(|(output, area)| output)
            .ok_or_else(|| anyhow!("window frame doesn't intersect with any display"))
    }
}
