use crate::{
    geometry,
    conn::Connection,
};

use {
    std::{
        rc::Rc,
        cell::{ Ref, RefCell },
    },
    xcb::{
        randr as xrandr,
        xproto,
        base as xbase,
    },
    anyhow::{ anyhow, Error },
};

pub trait ScreenLayoutable {
    fn get_screen_box(&self) -> geometry::ScreenBox;
    fn set_screen_box(&mut self, new: geometry::ScreenBox);
}

pub type CachedResult<T> = Rc<Result<T, Error>>;

pub struct Output {
    pub cm: Rc<Connection>,
    pub xoutput: xrandr::Output,
    info: RefCell<Option<CachedResult<xrandr::GetOutputInfoReply>>>,
    crtc_info: RefCell<Option<CachedResult<xrandr::GetCrtcInfoReply>>>,
    frame: RefCell<Option<CachedResult<geometry::ScreenBox>>>,
}

impl Output {
    pub fn new(connection: Rc<Connection>, xout: xrandr::Output) -> Self {
        Output {
            cm: connection,
            xoutput: xout,
            frame: RefCell::new(None),
            info: RefCell::new(None),
            crtc_info: RefCell::new(None),
        }
    }

    pub fn get_info(
        &self,
        timestamp: xproto::Timestamp,
    ) -> CachedResult<xrandr::GetOutputInfoReply> {
        {
            {
                let mut info = self.info.borrow_mut();

                if info.is_none() || info.as_ref().unwrap().is_err() {
                    *info = Some(Rc::new(
                        xrandr::get_output_info(&self.cm.connection, self.xoutput, timestamp)
                            .get_reply()
                            .map_err(|e| anyhow!("Couldn't get output info"))
                    ));
                }
            }
        }

        self.info.borrow().as_ref().unwrap().clone()
    }

    pub fn get_crtc_info(&self, timestamp: xproto::Timestamp) -> CachedResult<xrandr::GetCrtcInfoReply> {
        {
            let mut crtc_info = self.crtc_info.borrow_mut();
            if crtc_info.is_none() || crtc_info.as_ref().unwrap().is_err() {
                *crtc_info = match self.get_info(timestamp).as_ref() {
                    Ok(info) => {
                        Some(Rc::new(
                            xrandr::get_crtc_info(&self.cm.connection, info.crtc(), timestamp)
                                .get_reply()
                                .map_err(|e| anyhow!("Failed to get CRTC info"))
                        ))
                    },
                    Err(e) => None
                }
            }
        }

        self.crtc_info.borrow().as_ref().unwrap().clone()
    }


    pub fn get_frame(
        &self,
        timestamp: xproto::Timestamp,
    ) -> CachedResult<geometry::ScreenBox> {
        {
            let mut frame = self.frame.borrow_mut();
            if frame.is_none() || frame.as_ref().unwrap().is_err() {
                *frame = match self.get_crtc_info(timestamp).as_ref() {
                    Ok(crtc) => Some(Rc::new(Ok(
                        geometry::ScreenBox::new(
                            geometry::XPoint::new(crtc.x() as i32, crtc.y() as i32),
                            geometry::XPoint::new(crtc.x() as i32 + crtc.width() as i32, crtc.y() as i32 + crtc.height() as i32),
                        )))
                    ),
                    Err(e) => None
                }
            }
        }
        
        self.frame.borrow().as_ref().unwrap().clone()
    }
}