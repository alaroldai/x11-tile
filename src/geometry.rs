use {
    euclid::*,
};

pub struct XSpace;
pub type XPoint = Point2D<i32, XSpace>;
pub type XSize = Size2D<i32, XSpace>;
pub type ScreenBox = Box2D<i32, XSpace>;

pub struct OutputSpace;
pub type OutputPoint = Point2D<i32, OutputSpace>;
pub type OutputSize = Size2D<i32, OutputSpace>;
pub type OutputBox = Box2D<i32, OutputSpace>;