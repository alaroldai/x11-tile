mod xcb_util;

use log::debug;

use crate::xcb_util::{
  geometry::*,
  window::WindowExt,
};

use std::str;

use anyhow::{
  anyhow,
  Error,
};
use structopt::StructOpt;
use xcb::{
  base as xbase,
  randr as xrandr,
  xproto,
};

#[derive(StructOpt)]
struct GlobalOptions {}

#[derive(StructOpt)]
struct Fract {
  num: f32,
  denom: f32,
}

impl Fract {
  fn value(&self) -> f32 { self.num / self.denom }
}

impl std::str::FromStr for Fract {
  type Err = Error;
  fn from_str(s: &str) -> Result<Fract, Error> {
    let parts = s.split('/').collect::<Vec<_>>();
    Ok(Fract {
      num: f32::from_str(parts[0])?,
      denom: f32::from_str(parts[1])?,
    })
  }
}

struct Geometry<'a> {
  pub setup: xproto::Setup<'a>,
  pub root_win: xproto::Window,
  pub root_win_frame: ScreenRect,
  pub srs: xrandr::GetScreenResourcesCurrentReply,
  pub display_frames: Vec<ScreenRect>,
  pub work_areas: Vec<ScreenRect>,
  pub active_window: xproto::Window,
  pub active_window_frame: ScreenRect,
  pub active_window_insets: ScreenInsets,
}

fn get_geometry(conn: &xbase::Connection) -> Result<Geometry, Error> {
  let setup = conn.get_setup();

  let screen = setup
    .roots()
    .next()
    .ok_or_else(|| anyhow!("Couldn't unwrap screen 0"))?;

  let root_window = screen.root();

  let root_window_rect = root_window.get_geometry(&conn)?.as_rect();

  let srs = root_window.get_screen_resources_current(&conn)?;
  let timestamp = srs.config_timestamp();

  let display_frames = srs
    .outputs()
    .iter()
    .filter_map(|o| {
      let info = xrandr::get_output_info(&conn, *o, timestamp)
        .get_reply()
        .ok()?;
      match info.connection() as u32 {
        xrandr::CONNECTION_CONNECTED => {
          let crtc = xrandr::get_crtc_info(&conn, info.crtc(), timestamp)
            .get_reply()
            .ok()?;
          Some(crtc.as_rect())
        }
        _ => None,
      }
    })
    .collect();

  debug!("display_frames: {:?}", display_frames);

  let gvec: Vec<i32> =
    root_window.get_property(&conn, "_NET_WORKAREA", xproto::ATOM_CARDINAL, 8)?;

  debug!("gvec: {:?}", gvec);

  let work_area = gvec
    .as_slice()
    .chunks(4)
    .map(|slc| {
      ScreenRect::new(
        ScreenPoint::new(slc[0] as i32, slc[1] as i32),
        ScreenSize::new(slc[2] as i32, slc[3] as i32),
      )
    })
    .collect::<Vec<ScreenRect>>();

  debug!("Work area: {:?}", work_area);

  use xcb_util::geometry::*;

  let active_window: xproto::Window =
    root_window.get_property(&conn, "_NET_ACTIVE_WINDOW", xproto::ATOM_WINDOW, 1)?[0];

  let mut active_window_frame = active_window.get_geometry(&conn)?.as_rect();

  let translated =
    xproto::translate_coordinates(&conn, active_window, root_window, 0, 0).get_reply()?;
  active_window_frame.origin.x = translated.dst_x() as i32;
  active_window_frame.origin.y = translated.dst_y() as i32;

  let insets = active_window.get_property(&conn, "_NET_FRAME_EXTENTS", xproto::ATOM_CARDINAL, 4)?;
  let insets = if let [left, right, top, bottom] = insets.as_slice() {
    ScreenInsets::new(*top, *right, *bottom, *left)
  } else {
    ScreenInsets::zero()
  };

  Ok(Geometry {
    setup,
    root_win: root_window,
    root_win_frame: root_window_rect,
    srs,
    display_frames,
    work_areas: work_area,
    active_window,
    active_window_frame,
    active_window_insets: insets,
  })
}

#[derive(StructOpt)]
struct MoveWindowOnOutput {
  x: Fract,
  y: Fract,
  w: Fract,
  h: Fract,
}

fn inset_frame_by_struts(conn: &xbase::Connection, mut frame: ScreenRect, root_window: xproto::Window) -> Result<ScreenRect, Error> {
  let mut queue = vec![root_window];
  while let Some(w) = queue.pop() {
    let strut: Vec<i32> =
      w.get_property(conn, "_NET_WM_STRUT_PARTIAL", xproto::ATOM_CARDINAL, 12)?;
    if !strut.is_empty() {
      #[derive(Debug)]
      struct Strut {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
        left_start_y: i32,
        left_end_y: i32,
        right_start_y: i32,
        right_end_y: i32,
        top_start_x: i32,
        top_end_x: i32,
        bottom_start_x: i32,
        bottom_end_x: i32,
      }

      let strut = Strut {
        left: strut[0],
        right: strut[1],
        top: strut[2],
        bottom: strut[3],
        left_start_y: strut[4],
        left_end_y: strut[5],
        right_start_y: strut[6],
        right_end_y: strut[7],
        top_start_x: strut[8],
        top_end_x: strut[9],
        bottom_start_x: strut[10],
        bottom_end_x: strut[11],
      };

      // TODO:
      //  - Check if the strut-lines (NOT the whole rect) are contained within the
      //    target display frame
      //  - IF so, adjust the display frame

      if strut.top > frame.origin.y
        && strut.top < frame.origin.y + frame.size.height
        && strut.top_start_x >= frame.origin.x
        && strut.top_end_x <= frame.origin.x + frame.size.width
      {
        let overlap = strut.top - frame.origin.y;
        debug!("Found strut (overlap: {}): {:#?}", overlap, strut);
        frame.origin.y += overlap;
        frame.size.height -= overlap;
      }

      if strut.left > frame.origin.x
        && strut.left < frame.origin.x + frame.size.width
        && strut.left_start_y >= frame.origin.y
        && strut.left_end_y <= frame.origin.y + frame.size.height
      {
        let overlap = strut.left - frame.origin.x;
        debug!("Found strut (overlap: {}): {:#?}", overlap, strut);
        frame.origin.x += overlap;
        frame.size.width -= overlap;
      }

      if strut.bottom < frame.origin.y + frame.size.height
        && strut.bottom > frame.origin.y
        && strut.bottom_start_x >= frame.origin.x
        && strut.bottom_end_x <= frame.origin.x + frame.size.width
      {
        let overlap = frame.origin.y + frame.size.height - strut.bottom;
        debug!("Found strut (overlap: {}): {:#?}", overlap, strut);
        frame.size.height -= overlap;
      }

      if strut.right < frame.origin.x + frame.size.width
        && strut.right > frame.origin.x
        && strut.right_start_y >= frame.origin.y
        && strut.right_end_y <= frame.origin.y + frame.size.height
      {
        let overlap = frame.origin.x + frame.size.width - strut.left;
        debug!("Found strut (overlap: {}): {:#?}", overlap, strut);
        frame.size.width -= overlap;
      }
    }
    let mut children = xproto::query_tree(conn, w).get_reply()?.children().to_vec();

    queue.append(&mut children);
  }

  Ok(frame)
}

//  TODO (alaroldai):
//  Compute "output dimensions" by:
//  - Getting the rects of connected outputs
//  - Finding all windows that set the _NET_STRUT_PARTIAL
//    - FOR EACH, Inset the rect of the containing output if necessary
//  - Return the inset outputs.
fn get_output_available_rect(conn: &xbase::Connection) -> Result<ScreenRect, Error> {
  let setup = conn.get_setup();

  let screen = setup
    .roots()
    .next()
    .ok_or_else(|| anyhow!("Couldn't unwrap screen 0"))?;

  let root_window = screen.root();

  let active_window: xproto::Window =
    root_window.get_property(&conn, "_NET_ACTIVE_WINDOW", xproto::ATOM_WINDOW, 1)?[0];

  let mut active_window_frame = dbg!(active_window.get_geometry(&conn)?.as_rect());

  let translated =
    xproto::translate_coordinates(&conn, active_window, root_window, 0, 0).get_reply()?;
  active_window_frame.origin.x = translated.dst_x() as i32;
  active_window_frame.origin.y = translated.dst_y() as i32;

  let srs = root_window.get_screen_resources_current(&conn)?;
  let timestamp = srs.config_timestamp();

  let mut display_frame = srs
    .outputs()
    .iter()
    .filter_map(|o| {
      let info = xrandr::get_output_info(&conn, *o, timestamp)
        .get_reply()
        .ok()?;
      match info.connection() as u32 {
        xrandr::CONNECTION_CONNECTED => {
          let crtc = xrandr::get_crtc_info(&conn, info.crtc(), timestamp)
            .get_reply()
            .ok()?;
          Some(crtc.as_rect())
        }
        _ => None,
      }
    })
    .fold(None, |init: Option<ScreenRect>, frame| {
      let new = frame.intersection(&active_window_frame);
      debug!(
        "{}: {} intersection with {}",
        frame,
        if new.is_some() { "Some" } else { "No" },
        active_window_frame
      );
      match (new, init) {
        (Some(new), Some(old)) if new.area() > old.area() => Some(frame),
        (Some(_), None) => Some(frame),
        _ => init,
      }
    })
    .unwrap();

  display_frame = inset_frame_by_struts(conn, display_frame, root_window)?;

  Ok(display_frame)
}

impl MoveWindowOnOutput {
  fn run(self, _: GlobalOptions) -> Result<(), Error> {
    let (conn, _) = xbase::Connection::connect(None)?;
    let display_frame = get_output_available_rect(&conn)?;

    let geom = get_geometry(&conn)?;

    let pct = DisplayPercentageSpaceRect::new(
      DisplayPercentageSpacePoint::new(self.x.value(), self.y.value()),
      DisplayPercentageSpaceSize::new(self.w.value(), self.h.value()),
    );

    let new_rect = pct
      .to_rect(display_frame)
      .inner_rect(geom.active_window_insets);

    dbg!(&new_rect);

    // NOTE: Some window managers (Kwin and XFWM, for example) may refuse to
    // position windows as requested if they are in a "tiled" or "maximised"
    // state. In the case of Kwin, this can be fixed by using a window rule to
    // force the "ignore requested geometry" flag to `false`.
    geom
      .root_win
      .move_resize(&conn, geom.active_window, new_rect)?;

    Ok(())
  }
}

#[derive(StructOpt)]
enum Direction {
  North,
  South,
  East,
  West,
}

impl std::str::FromStr for Direction {
  type Err = Error;
  fn from_str(s: &str) -> Result<Direction, Error> {
    match s {
      "h" => Ok(Direction::West),
      "j" => Ok(Direction::South),
      "k" => Ok(Direction::North),
      "l" => Ok(Direction::East),
      _ => Err(anyhow!("Not a known direction - use hjkl")),
    }
  }
}

#[derive(StructOpt)]
struct MoveWindowToOutput {
  direction: Direction,
}

impl MoveWindowToOutput {
  fn run(self, _: GlobalOptions) -> Result<(), Error> {
    let (conn, _) = xbase::Connection::connect(None)?;

    let mut geom = get_geometry(&conn)?;

    let (x, y) = match self.direction {
      Direction::West => (-1.0, 0.0),
      Direction::South => (0.0, 1.0),
      Direction::North => (0.0, -1.0),
      Direction::East => (1.0, 0.0),
    };

    let direction: euclid::Vector2D<f32, ScreenSpace> = euclid::Vector2D::new(x as f32, y as f32);

    let current_output_frame = geom
      .display_frames
      .iter()
      .fold(None, |init: Option<ScreenRect>, frame| {
        let new = frame.intersection(&geom.active_window_frame);
        println!("Found intersection: {:#?}", new);
        match (new, init) {
          (Some(new), Some(old)) if new.area() > old.area() => Some(*frame),
          (Some(_), None) => Some(*frame),
          _ => init,
        }
      })
      .and_then(|frame| inset_frame_by_struts(&conn, frame, geom.root_win).ok())
      .unwrap();

    let new_output_frame = geom
      .display_frames
      .iter()
      .fold(None, |init: Option<ScreenRect>, frame| {
        let vec: euclid::Vector2D<f32, ScreenSpace> =
          (frame.center() - current_output_frame.center()).cast::<f32>();
        let old: Option<euclid::Vector2D<f32, ScreenSpace>> =
          init.map(|init| (init.center() - current_output_frame.center()).cast::<f32>());

        let projection = vec.dot(direction);

        match old {
          None if projection > 0.0 => {
            println!(
              "Starting with output {:#?} / projection {:#?}",
              frame, projection
            );
            Some(*frame)
          }
          Some(old) if projection < old.dot(direction) && projection > 0.0 => {
            println!(
              "Replacing projection {} ({}) with {} ({})",
              init.unwrap(),
              old.dot(direction),
              frame,
              projection
            );
            Some(*frame)
          }
          _ => {
            println!(
              "Ignoring output {:#?} with projection {:#?}",
              frame, projection
            );
            init
          }
        }
      })
      .unwrap();

    let new_output_frame = inset_frame_by_struts(&conn, new_output_frame, geom.root_win)?;

    dbg!(&geom.active_window_frame);


    // geom.active_window_frame = geom.active_window_frame.inner_rect(geom.active_window_insets);
    dbg!(&geom.active_window_insets);
    dbg!(&current_output_frame);
    dbg!(&new_output_frame);

    let decorated_source_frame = geom.active_window_frame.outer_rect(geom.active_window_insets);
    let pct_rect = decorated_source_frame.as_dps(current_output_frame);

    dbg!(&pct_rect);

    let decorated_dest_frame = pct_rect.to_rect(new_output_frame);
    let bare_dest_frame = decorated_dest_frame.inner_rect(geom.active_window_insets);

    dbg!(&bare_dest_frame);

    geom
      .root_win
      .move_resize(&conn, geom.active_window, bare_dest_frame)
  }
}

fn main() -> Result<(), Error> {
  env_logger::init();

  #[derive(StructOpt)]
  enum Action {
    MoveWindowOnOutput(MoveWindowOnOutput),
    MoveWindowToOutput(MoveWindowToOutput),
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
        Action::MoveWindowOnOutput(opts) => opts.run(self.options),
        Action::MoveWindowToOutput(opts) => opts.run(self.options),
      }
    }
  }

  App::from_args().run()
}
