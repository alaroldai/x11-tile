use crate::{
    conn::{Connection},
    output,
};

use {
    std::{
        rc::Rc,
    },
    xcb::{
        randr as xrandr,
        xproto,
        base as xbase,
    }
};

pub struct ScreenResources {
    pub cm: Rc<Connection>,
    pub xsr: xrandr::GetScreenResourcesCurrentReply,
}

impl<'a> ScreenResources {
    pub fn get_outputs(&self) -> Vec<output::Output> {
        self.xsr.outputs()
        .iter()
        .map(|xoutput| { output::Output::new(self.cm.clone(), *xoutput) })
        .collect()
    }
}