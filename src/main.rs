mod xcb_util;

use crate::xcb_util::{geometry::*, window::WindowExt};

use std::str;

use {
  anyhow::{anyhow, Error},
  structopt::StructOpt,
  xcb::{base as xbase, randr as xrandr, xproto},
};

#[derive(StructOpt)]
struct GlobalOptions {}

#[derive(StructOpt)]
struct Fract {
  num: f32,
  denom: f32,
}

impl Fract {
  fn value(&self) -> f32 {
    self.num / self.denom
  }
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

fn get_geometry(conn: & xbase::Connection) -> Result<Geometry, Error> {
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

  let gvec: Vec<i32> =
    root_window.get_property(&conn, "_NET_WORKAREA", xproto::ATOM_CARDINAL, 4)?;

  let work_area = gvec
    .as_slice()
    .chunks(4)
    .map(|slc| {
      ScreenRect::new(
        ScreenPoint::new(slc[0] as i32, slc[1] as i32),
        ScreenSize::new(slc[2] as i32, slc[3] as i32),
      )
    })
    .collect::<Vec<ScreenRect>>()[0];

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
    work_areas: vec![work_area],
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

impl MoveWindowOnOutput {
  fn run(self, _: GlobalOptions) -> Result<(), Error> {
    let (conn, _) = xbase::Connection::connect(None)?;

    let geom = get_geometry(&conn)?;

    let current_display = geom
      .display_frames
      .iter()
      .fold(None, |init: Option<ScreenRect>, frame| {
        let new = frame.intersection(&geom.active_window_frame);
        match (new, init) {
          (Some(new), Some(old)) if new.area() > old.area() => Some(*frame),
          (Some(_), None) => Some(*frame),
          _ => init,
        }
      })
      .unwrap();

    let display_frame = current_display.intersection(&geom.work_areas[0]).unwrap();

    let pct = DisplayPercentageSpaceRect::new(
      DisplayPercentageSpacePoint::new(self.x.value(), self.y.value()),
      DisplayPercentageSpaceSize::new(self.w.value(), self.h.value()),
    );

    let new_rect = pct
      .to_rect(display_frame)
      .inner_rect(geom.active_window_insets);

    dbg!(&new_rect);

    // NOTE: Some window managers (Kwin and XFWM, for example) may refuse to position windows as requested
    // if they are in a "tiled" or "maximised" state.
    // In the case of Kwin, this can be fixed by using a window rule to force the "ignore requested geometry" flag to `false`.
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

    let geom = get_geometry(&conn)?;

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
      .and_then(|frame| frame.intersection(&geom.work_areas[0]))
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

    let new_output_frame = new_output_frame.intersection(&geom.work_areas[0]).unwrap();

    dbg!(&geom.active_window_frame);

    dbg!(&current_output_frame);
    dbg!(&new_output_frame);

    let pct_rect = geom.active_window_frame.as_dps(current_output_frame);

    dbg!(&pct_rect);

    let new_rect = pct_rect.to_rect(new_output_frame);

    dbg!(&new_rect);

    geom
      .root_win
      .move_resize(&conn, geom.active_window, new_rect)
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
