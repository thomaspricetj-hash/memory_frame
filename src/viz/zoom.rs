#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomLevel {
    Frame,
    Slice,
    Grid,
    Cell,
}

impl ZoomLevel {
    /// Next zoom level (clamped at Cell)
    pub fn next(self) -> Self {
        match self {
            ZoomLevel::Frame => ZoomLevel::Slice,
            ZoomLevel::Slice => ZoomLevel::Grid,
            ZoomLevel::Grid  => ZoomLevel::Cell,
            ZoomLevel::Cell  => ZoomLevel::Cell,
        }
    }

    /// Previous zoom level (clamped at Frame)
    pub fn prev(self) -> Self {
        match self {
            ZoomLevel::Cell  => ZoomLevel::Grid,
            ZoomLevel::Grid  => ZoomLevel::Slice,
            ZoomLevel::Slice => ZoomLevel::Frame,
            ZoomLevel::Frame => ZoomLevel::Frame,
        }
    }

    /// Numeric representation (useful for UI scaling)
    pub fn as_index(self) -> usize {
        match self {
            ZoomLevel::Frame => 0,
            ZoomLevel::Slice => 1,
            ZoomLevel::Grid  => 2,
            ZoomLevel::Cell  => 3,
        }
    }

    /// Humanâ€‘readable label
    pub fn label(self) -> &'static str {
        match self {
            ZoomLevel::Frame => "Frame View",
            ZoomLevel::Slice => "Slice View",
            ZoomLevel::Grid  => "Grid View",
            ZoomLevel::Cell  => "Cell View",
        }
    }

    /// Is this the most zoomedâ€‘in level?
    pub fn is_max(self) -> bool {
        self == ZoomLevel::Cell
    }

    /// Is this the most zoomedâ€‘out level?
    pub fn is_min(self) -> bool {
        self == ZoomLevel::Frame
    }
}

#[derive(Debug)]
pub struct ZoomController {
    pub level: ZoomLevel,
}

impl ZoomController {
    pub fn new() -> Self {
        Self { level: ZoomLevel::Frame }
    }

    /// Zoom in one level (clamped)
    pub fn zoom_in(&mut self) {
        self.level = self.level.next();
    }

    /// Zoom out one level (clamped)
    pub fn zoom_out(&mut self) {
        self.level = self.level.prev();
    }

    /// Jump directly to a specific level
    pub fn set(&mut self, level: ZoomLevel) {
        self.level = level;
    }

    /// Reset to the outermost view
    pub fn reset(&mut self) {
        self.level = ZoomLevel::Frame;
    }

    /// Humanâ€‘readable label for UI
    pub fn label(&self) -> &'static str {
        self.level.label()
    }

    /// Numeric index (0â€“3)
    pub fn index(&self) -> usize {
        self.level.as_index()
    }

    /// Are we fully zoomed in?
    pub fn at_max(&self) -> bool {
        self.level.is_max()
    }

    /// Are we fully zoomed out?
    pub fn at_min(&self) -> bool {
        self.level.is_min()
    }
}






