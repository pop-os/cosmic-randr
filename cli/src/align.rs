pub fn display<R: Rectangular>(new_region: &mut R, other_displays: impl Iterator<Item = R>) {
    let mut nearest = f32::MAX;
    let mut nearest_region = R::default();
    let mut nearest_side = NearestSide::East;

    // Find the nearest adjacent display to the display.
    for other_display in other_displays {
        let center = new_region.center();

        let eastward = distance(other_display.east_point(), center) * 1.25;
        let westward = distance(other_display.west_point(), center) * 1.25;
        let northward = distance(other_display.north_point(), center);
        let southward = distance(other_display.south_point(), center);

        let mut nearer = false;

        if nearest > eastward {
            (nearest, nearest_side, nearer) = (eastward, NearestSide::East, true);
        }

        if nearest > westward {
            (nearest, nearest_side, nearer) = (westward, NearestSide::West, true);
        }

        if nearest > northward {
            (nearest, nearest_side, nearer) = (northward, NearestSide::North, true);
        }

        if nearest > southward {
            (nearest, nearest_side, nearer) = (southward, NearestSide::South, true);
        }

        if nearer {
            nearest_region = other_display;
        }
    }

    // Attach display to nearest adjacent display.
    match nearest_side {
        NearestSide::East => {
            new_region.set_x(nearest_region.x() - new_region.width());
            new_region.set_y(
                new_region
                    .y()
                    .max(nearest_region.y() - new_region.height() + 4.0)
                    .min(nearest_region.y() + nearest_region.height() - 4.0),
            );
        }

        NearestSide::North => {
            new_region.set_y(nearest_region.y() - new_region.height());
            new_region.set_x(
                new_region
                    .x()
                    .max(nearest_region.x() - new_region.width() + 4.0)
                    .min(nearest_region.x() + nearest_region.width() - 4.0),
            );
        }

        NearestSide::West => {
            new_region.set_x(nearest_region.x() + nearest_region.width());
            new_region.set_y(
                new_region
                    .y()
                    .max(nearest_region.y() - new_region.height() + 4.0)
                    .min(nearest_region.y() + nearest_region.height() - 4.0),
            );
        }

        NearestSide::South => {
            new_region.set_y(nearest_region.y() + nearest_region.height());
            new_region.set_x(
                new_region
                    .x()
                    .max(nearest_region.x() - new_region.width() + 4.0)
                    .min(nearest_region.x() + nearest_region.width() - 4.0),
            );
        }
    }

    // Snap-align on x-axis when alignment is near.
    if (new_region.x() - nearest_region.x()).abs() <= 4.0 {
        new_region.set_x(nearest_region.x());
    }

    // Snap-align on x-axis when alignment is near bottom edge.
    if ((new_region.x() + new_region.width()) - (nearest_region.x() + nearest_region.width())).abs()
        <= 4.0
    {
        new_region.set_x(nearest_region.x() + nearest_region.width() - new_region.width());
    }

    // Snap-align on y-axis when alignment is near.
    if (new_region.y() - nearest_region.y()).abs() <= 4.0 {
        new_region.set_y(nearest_region.y());
    }

    // Snap-align on y-axis when alignment is near bottom edge.
    if ((new_region.y() + new_region.height()) - (nearest_region.y() + nearest_region.height()))
        .abs()
        <= 4.0
    {
        new_region.set_y(nearest_region.y() + nearest_region.height() - new_region.height());
    }
}

fn distance(a: Point, b: Point) -> f32 {
    ((b.x - a.x).powf(2.0) + (b.y - a.y).powf(2.0)).sqrt()
}

#[derive(Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub enum NearestSide {
    East,
    North,
    South,
    West,
}

#[derive(Default)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rectangular for Rectangle {
    fn x(&self) -> f32 {
        self.x
    }

    fn set_x(&mut self, x: f32) {
        self.x = x;
    }

    fn y(&self) -> f32 {
        self.y
    }

    fn set_y(&mut self, y: f32) {
        self.y = y;
    }

    fn width(&self) -> f32 {
        self.width
    }

    fn set_width(&mut self, width: f32) {
        self.width = width;
    }

    fn height(&self) -> f32 {
        self.height
    }

    fn set_height(&mut self, height: f32) {
        self.height = height;
    }
}

pub trait Rectangular: Default + Sized {
    fn x(&self) -> f32;

    fn set_x(&mut self, x: f32);

    fn y(&self) -> f32;

    fn set_y(&mut self, y: f32);

    fn width(&self) -> f32;

    fn set_width(&mut self, width: f32);

    fn height(&self) -> f32;

    fn set_height(&mut self, height: f32);

    fn center(&self) -> Point {
        Point {
            x: self.center_x(),
            y: self.center_y(),
        }
    }

    fn center_x(&self) -> f32 {
        self.x() + self.width() / 2.0
    }

    fn center_y(&self) -> f32 {
        self.y() + self.height() / 2.0
    }

    fn east_point(&self) -> Point {
        Point {
            x: self.x(),
            y: self.center_y(),
        }
    }

    fn north_point(&self) -> Point {
        Point {
            x: self.center_x(),
            y: self.y(),
        }
    }

    fn west_point(&self) -> Point {
        Point {
            x: self.x() + self.width(),
            y: self.center_y(),
        }
    }

    fn south_point(&self) -> Point {
        Point {
            x: self.center_x(),
            y: self.y() + self.height(),
        }
    }
}
