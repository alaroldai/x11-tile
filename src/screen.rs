use crate::{
    geometry,
    conn::Connection,
};

use {
    xcb::{
        xproto,
    },
    std::{
        rc::Rc,
    },
};

pub struct Screen<'a> {
    pub cm: Rc<Connection>,
    pub xscreen: xproto::Screen<'a>,
}

