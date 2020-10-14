use {
  euclid::*,
  xcb::{randr as xrandr, xproto},
};

pub struct ScreenSpace;
pub type ScreenPoint = Point2D<i32, ScreenSpace>;
pub type ScreenSize = Size2D<i32, ScreenSpace>;
pub type ScreenRect = Rect<i32, ScreenSpace>;
pub type ScreenInsets = SideOffsets2D<i32, ScreenSpace>;

pub struct DisplayPercentageSpace;
pub type DisplayPercentageSpacePoint = Point2D<f32, DisplayPercentageSpace>;
pub type DisplayPercentageSpaceSize = Size2D<f32, DisplayPercentageSpace>;
pub type DisplayPercentageSpaceRect = Rect<f32, DisplayPercentageSpace>;

pub trait AsScreenRect {
  fn as_rect(&self) -> ScreenRect;
}

impl AsScreenRect for xproto::GetGeometryReply {
  fn as_rect(&self) -> ScreenRect {
    ScreenRect::new(
      ScreenPoint::new(self.x() as i32, self.y() as i32),
      ScreenSize::new(self.width() as i32, self.height() as i32),
    )
  }
}

impl AsScreenRect for xrandr::GetCrtcInfoReply {
  fn as_rect(&self) -> ScreenRect {
    ScreenRect::new(
      ScreenPoint::new(self.x() as i32, self.y() as i32),
      ScreenSize::new(self.width() as i32, self.height() as i32),
    )
  }
}

pub trait ToDisplayPercentageSpace {
  fn as_dps(&self, display: Self) -> DisplayPercentageSpaceRect;
}

impl ToDisplayPercentageSpace for ScreenRect {
  fn as_dps(&self, display: Self) -> DisplayPercentageSpaceRect {
    let origin = self.origin - display.origin.to_vector();
    DisplayPercentageSpaceRect::new(
      DisplayPercentageSpacePoint::new(
        origin.x as f32 / display.size.width as f32,
        origin.y as f32 / display.size.height as f32,
      ),
      DisplayPercentageSpaceSize::new(
        self.size.width as f32 / display.size.width as f32,
        self.size.height as f32 / display.size.height as f32,
      ),
    )
  }
}

pub trait ToScreenRect {
  fn to_rect(&self, display: ScreenRect) -> ScreenRect;
}

impl ToScreenRect for DisplayPercentageSpaceRect {
  fn to_rect(&self, display: ScreenRect) -> ScreenRect {
    ScreenRect::new(
      display.origin
        + ScreenPoint::new(
          (self.origin.x * (display.width() as f32)) as i32,
          (self.origin.y * (display.height() as f32)) as i32,
        )
        .to_vector(),
      ScreenSize::new(
        (self.width() * (display.width() as f32)) as i32,
        (self.height() * (display.height() as f32)) as i32,
      ),
    )
  }
}
