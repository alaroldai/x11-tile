mod conn;
mod geometry;
mod window;
mod output;
mod screen;
mod screen_resources;

use {
    anyhow::{anyhow, Error},
    log::trace,
    structopt::StructOpt,
    xcb::{base as xbase, randr as xrandr, xproto},
};

use crate::conn::*;

#[derive(StructOpt)]
struct GlobalOptions {}

mod placement {

    use {
        anyhow::{anyhow, Error},
        structopt::StructOpt,
    };

    #[derive(StructOpt)]
    pub enum Edge {
        Top,
        Bottom,
        Left,
        Right,
    }

    impl std::str::FromStr for Edge {
        type Err = Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "top" => Ok(Self::Top),
                "bottom" => Ok(Self::Bottom),
                "left" => Ok(Self::Left),
                "right" => Ok(Self::Right),
                _ => Err(anyhow!("Couldn't parse")),
            }
        }
    }

    #[derive(StructOpt)]
    pub enum Corner {
        TopLeft,
        TopRight,
        BottomLeft,
        BottomRight,
    }

    impl std::str::FromStr for Corner {
        type Err = Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "top-left" => Ok(Self::TopLeft),
                "top-right" => Ok(Self::TopRight),
                "bottom-left" => Ok(Self::BottomLeft),
                "bottom-right" => Ok(Self::BottomRight),
                _ => Err(anyhow!("Couldn't parse")),
            }
        }
    }

    #[derive(StructOpt)]
    pub enum Quarter {
        Edge(Edge),
        Corner(Corner),
        Centre,
    }

    impl std::str::FromStr for Quarter {
        type Err = Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.starts_with("edge-") {
                Ok(Self::Edge(Edge::from_str(s.trim_start_matches("edge-"))?))
            } else if s.starts_with("corner-") {
                Ok(Self::Corner(Corner::from_str(
                    s.trim_start_matches("corner-"),
                )?))
            } else if s == "centre" {
                Ok(Self::Centre)
            } else {
                Err(anyhow!("Couldn't parse"))
            }
        }
    }

    #[derive(StructOpt)]
    pub enum NinthColumn {
        Left,
        Centre,
        Right,
    }

    impl std::str::FromStr for NinthColumn {
        type Err = Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "left" => Ok(Self::Left),
                "centre" => Ok(Self::Centre),
                "right" => Ok(Self::Right),
                _ => Err(anyhow!("Couldn't parse")),
            }
        }
    }

    #[derive(StructOpt)]
    pub enum NinthRow {
        Top,
        Centre,
        Bottom,
    }

    impl std::str::FromStr for NinthRow {
        type Err = Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "top" => Ok(Self::Top),
                "centre" => Ok(Self::Centre),
                "bottom" => Ok(Self::Bottom),
                _ => Err(anyhow!("Couldn't parse")),
            }
        }
    }

    #[derive(StructOpt)]
    pub enum Ninth {
        Edge(Edge),
        Corner(Corner),
        Column(NinthColumn),
        Row(NinthRow),
        Centre,
    }

    impl std::str::FromStr for Ninth {
        type Err = Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.starts_with("edge-") {
                Ok(Self::Edge(Edge::from_str(s.trim_start_matches("edge-"))?))
            } else if s.starts_with("corner-") {
                Ok(Self::Corner(Corner::from_str(
                    s.trim_start_matches("corner-"),
                )?))
            } else if s.starts_with("column-") {
                Ok(Self::Column(NinthColumn::from_str(
                    s.trim_start_matches("column-"),
                )?))
            } else if s.starts_with("row-") {
                Ok(Self::Row(NinthRow::from_str(s.trim_start_matches("row-"))?))
            } else if s == "centre" {
                Ok(Self::Centre)
            } else {
                Err(anyhow!("Couldn't parse"))
            }
        }
    }

    #[derive(StructOpt)]
    pub enum Placement {
        Half(Edge),
        Quarter(Quarter),
        Ninth(Ninth),
    }

    impl std::str::FromStr for Placement {
        type Err = Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.starts_with("half-") {
                Ok(Self::Half(Edge::from_str(s.trim_start_matches("half-"))?))
            } else if s.starts_with("quarter-") {
                Ok(Self::Quarter(Quarter::from_str(
                    s.trim_start_matches("quarter-"),
                )?))
            } else if s.starts_with("ninth-") {
                Ok(Self::Ninth(Ninth::from_str(
                    s.trim_start_matches("ninth-"),
                )?))
            } else {
                Err(anyhow!("Couldn't parse"))
            }
        }
    }
}

// #[derive(StructOpt)]
// struct MoveWindowToOutput {
//     target: placement::Edge,
// }

// impl MoveWindowToOutput {
//     fn run(self, options: GlobalOptions) -> Result<(), Error> {
//         use std::cmp::Ordering;

//         let cm = Connection::new()?;
//         let root_window = cm.root_window()?;
//         let resources = root_window.get_screen_resources_current()?;

//         let screen = cm.root_screen()?;
//         let screen_frame = geometry::ScreenBox::from_size(
//             geometry::XSize::new(
//                 screen.width_in_pixels() as i32,
//                 screen.height_in_pixels() as i32,
//             ),
//         );

//         let active_window = root_window.get_active_window()?;

//         println!(
//             "Will move window: {}",
//             std::str::from_utf8(&active_window.get_property(
//                 "_NET_WM_NAME",
//                 cm.get_atom("UTF8_STRING")?,
//                 512
//             )?)?
//         );

//         let geom = active_window.get_geometry()?;
//         let active_frame = geometry::ScreenBox::new(
//             geometry::XPoint::new(
//                 geom.x() as i32,
//                 geom.y() as i32,
//             ),
//             geometry::XPoint::new(
//                 geom.x() as i32 + geom.width() as i32,
//                 geom.y() as i32 + geom.height() as i32,
//             ),
//         );

//         let current_output = cm.get_output_id_for_window(active_window.xwin, &resources)?;

//         let current_output_frame = cm.get_output_frame(&resources, &current_output)?;

//         let outputs = cm.get_output_frames(&resources)?;

//         let mut of = None;
//         for (output, frame) in outputs {
//             let info = cm.get_output_info(output, resources.config_timestamp())?;
//             let name = std::str::from_utf8(info.name())?;
//             if frame.min.x <= active_frame.max.x {
//                 println!("Ignoring {} (left-of / equal-to active frame)", name);
//                 continue;
//             }

//             let (hd, vd) = (
//                 frame.horizontal_distance_from(active_frame),
//                 frame.vertical_distance_from(active_frame),
//             );
//             of = match of {
//                 None => Some((output, frame)),
//                 Some((eo, ef)) => {
//                     let (ehd, evd) = (
//                         ef.horizontal_distance_from(active_frame),
//                         ef.vertical_distance_from(active_frame),
//                     );
//                     match ehd.cmp(&hd) {
//                         Ordering::Less => Some((eo, ef)),
//                         Ordering::Greater => Some((output, frame)),
//                         Ordering::Equal => match evd.cmp(&vd) {
//                             Ordering::Less => Some((eo, ef)),
//                             _ => Some((output, frame)),
//                         },
//                     }
//                 }
//             };
//         }

//         let (target, target_frame) = of.unwrap();

//         let info = cm.get_output_info(target, resources.config_timestamp())?;
//         let name = std::str::from_utf8(info.name())?;
//         println!("Selected {}", name);

//         let new_frame = screen_frame.location_of_rect_after_reparenting(
//             active_frame,
//             current_output_frame,
//             target_frame,
//         );

//         println!("Source rect: {:#?}", active_frame);
//         println!("Target rect: {:#?}", new_frame);

//         if !cm.supports("_NET_MOVERESIZE_WINDOW")? {
//             return Err(anyhow!("WM doesn't support _NET_MOVERESIZE_WINDOW"));
//         }

//         let flags = {
//             let mut result = 0;
//             // bits 8-11 are presence bits for x/y/w/h
//             /* if x != -1 */
//             {
//                 result |= 1 << 8;
//             }
//             /* if y != -1 */
//             {
//                 result |= 1 << 9;
//             }
//             /* if w != -1 */
//             {
//                 result |= 1 << 10;
//             }
//             /* if h != -1 */
//             {
//                 result |= 1 << 11;
//             }
//             result
//         };

//         let ev = xcb::ClientMessageEvent::new(
//             32,
//             active_window,
//             cm.get_type("_NET_MOVERESIZE_WINDOW")?,
//             xproto::ClientMessageData::from_data32([
//                 flags,
//                 new_frame.origin.x as u32,
//                 new_frame.origin.y as u32,
//                 new_frame.size.width,
//                 new_frame.size.height,
//             ]),
//         );

//         cm.send_event(
//             true,
//             root_window,
//             xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY | xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT,
//             &ev,
//         )?;

//         Ok(())
//     }
// }

#[derive(StructOpt)]
struct PrintOutputs {}

impl PrintOutputs {
    fn run(self, options: GlobalOptions) -> Result<(), Error> {
        let cm = Connection::new()?;
        let root_window = cm.root_window()?;
        let screen_resources = root_window.get_screen_resources_current()?;

        let print_output_info = |output: &output::Output| -> Result<(), Error> {
            let info = output.get_info(screen_resources.xsr.config_timestamp());
            if let Err(_) = info.as_ref() {
                return Err(anyhow!("???"));
            }

            let info = info.as_ref().as_ref().unwrap();
            // match info.as_ref() {
            //     Err(e) => 
            //         return Err(anyhow!("???")),
            //     Ok(info) => {
                    let name = std::str::from_utf8(info.name())?;
                    if info.connection() != xrandr::CONNECTION_CONNECTED as u8 {
                        println!("{}: Disconnected", name);
                        return Ok(());
                    }
                    let crtc = cm.get_crtc_info(info.crtc(), screen_resources.xsr.config_timestamp())?;
                    let frame = geometry::ScreenBox::new(
                        geometry::XPoint::new(
                            crtc.x() as i32,
                            crtc.y() as i32,
                        ),
                        geometry::XPoint::new(
                            crtc.x() as i32 + crtc.width() as i32,
                            crtc.y() as i32 + crtc.height() as i32,
                        ),
                    );

                    println!("{}: {:#?}", name, frame);
                // }
            // }

            Ok(())
        };

        for output in screen_resources.get_outputs() {
            if let Err(e) = print_output_info(&output) {
                println!("Error getting output info: {}", e)
            }
        }

        Ok(())
    }
}

#[derive(StructOpt)]
struct ResizeOnDisplay {}

impl ResizeOnDisplay {
    fn run(self, options: GlobalOptions) -> Result<(), Error> {
        trace!("Running ResizeOnDisplay");

        let cm = Connection::new()?;

        let root_window = cm.root_window()?;

        let active_window = window::Window {
            cm: cm.clone(),
            xwin: root_window.get_property("_NET_ACTIVE_WINDOW", xproto::ATOM_WINDOW, 1)?[0],
        };

        println!(
            "active window: {}",
            std::str::from_utf8(&active_window.get_property(
                "_NET_WM_NAME",
                cm.get_atom("UTF8_STRING")?,
                512
            )?)?
        );

        if !root_window.supports("_NET_MOVERESIZE_WINDOW")? {
            return Err(anyhow!("WM doesn't support _NET_MOVERESIZE_WINDOW"));
        }

        let (x, y, w, h) = (0, 0, 800, 600);
        let gravity_flags = {
            let mut result = 0;
            /* if x != -1 */
            {
                result |= 1 << 8;
            }
            /* if y != -1 */
            {
                result |= 1 << 9;
            }
            /* if w != -1 */
            {
                result |= 1 << 10;
            }
            /* if h != -1 */
            {
                result |= 1 << 11;
            }
            result
        };

        let ev = xcb::ClientMessageEvent::new(
            32,
            active_window.xwin,
            cm.get_atom("_NET_MOVERESIZE_WINDOW")?,
            xproto::ClientMessageData::from_data32([gravity_flags, x, y, w, h]),
        );

        root_window.send_event(
            false,
            xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY | xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT,
            &ev,
        )?;

        cm.flush();

        Ok(())
    }
}

#[derive(StructOpt)]
struct TestGetDisplays {}

impl TestGetDisplays {
    fn run(self, options: GlobalOptions) -> Result<(), Error> {
        trace!("Running TestGetDisplays");

        let cm = Connection::new()?;
        let root_window = cm.root_window()?;

        let client_list = root_window.get_property(
            "_NET_CLIENT_LIST",
            xproto::ATOM_WINDOW,
            10,
        )?.iter().map(|xwin| window::Window {
            cm: cm.clone(),
            xwin: *xwin,
        }).collect::<Vec<_>>();

        for window in client_list {
            let window_name_bytes: Vec<u8> =
                window.get_property("_NET_WM_NAME", cm.get_atom("UTF8_STRING")?, 512)?;
            println!("{}", std::str::from_utf8(&window_name_bytes)?);
        }

        Ok(())
    }
}

fn main() -> Result<(), Error> {
    env_logger::init();

    #[derive(StructOpt)]
    enum Action {
        TestGetDisplays(TestGetDisplays),
        ResizeOnDisplay(ResizeOnDisplay),
        PrintOutputs(PrintOutputs),
        // MoveWindowToOutput(MoveWindowToOutput),
    }

    #[derive(StructOpt)]
    struct App {
        #[structopt(flatten)]
        options: GlobalOptions,
        #[structopt(subcommand)]
        action: Action,
    }

    impl App {
        fn run(self) -> Result<(), Error> {
            match self.action {
                Action::TestGetDisplays(opts) => opts.run(self.options),
                Action::ResizeOnDisplay(opts) => opts.run(self.options),
                Action::PrintOutputs(opts) => opts.run(self.options),
                // Action::MoveWindowToOutput(opts) => opts.run(self.options),
            }
        }
    }

    App::from_args().run()
}
