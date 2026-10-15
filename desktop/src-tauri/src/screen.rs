use image::RgbaImage;

#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
}

pub fn primary_geometry() -> Result<Geometry, String> {
    let monitors = xcap::Monitor::all().map_err(|err| err.to_string())?;
    let primary = monitors
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| "no primary display".to_string())?;
    Ok(Geometry {
        width: primary.width().map_err(|err| err.to_string())?,
        height: primary.height().map_err(|err| err.to_string())?,
        origin_x: primary.x().map_err(|err| err.to_string())?,
        origin_y: primary.y().map_err(|err| err.to_string())?,
    })
}

pub fn cursor_position() -> Result<(i32, i32), String> {
    let geometry = primary_geometry()?;
    let (x, y) = raw_cursor()?;
    Ok((x - geometry.origin_x, y - geometry.origin_y))
}

pub struct Frame {
    pub device: RgbaImage,
    pub logical_width: u32,
    pub logical_height: u32,
}

impl Frame {
    pub fn grid(&self) -> Grid {
        Grid::from_logical(self.logical_width, self.logical_height)
    }

    pub fn logical_image(&self) -> RgbaImage {
        resize_image(&self.device, self.logical_width, self.logical_height)
    }

    pub fn shot_image(&self) -> RgbaImage {
        let grid = self.grid();
        resize_image(&self.device, grid.shot_width, grid.shot_height)
    }
}

pub fn capture_frame() -> Result<Frame, String> {
    let monitors = xcap::Monitor::all().map_err(|err| err.to_string())?;
    let primary = monitors
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| "no primary display".to_string())?;
    Ok(Frame {
        device: primary.capture_image().map_err(|err| err.to_string())?,
        logical_width: primary.width().map_err(|err| err.to_string())?,
        logical_height: primary.height().map_err(|err| err.to_string())?,
    })
}

/// Screenshot long edge and visual-token budget for computer-use models.
pub const MAX_LONG_EDGE: u32 = 2576;
pub const MAX_VISUAL_TOKENS: u32 = 4784;
const TOKEN_PATCH: u32 = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    pub shot_width: u32,
    pub shot_height: u32,
    pub logical_width: u32,
    pub logical_height: u32,
}

impl Grid {
    pub fn from_logical(logical_width: u32, logical_height: u32) -> Self {
        let (shot_width, shot_height) = fit_screenshot(logical_width, logical_height);
        Self {
            shot_width,
            shot_height,
            logical_width,
            logical_height,
        }
    }

    pub fn point_to_logical(&self, x: i32, y: i32) -> Result<(i32, i32), String> {
        if x < 0 || y < 0 || x as u32 >= self.shot_width || y as u32 >= self.shot_height {
            return Err(format!(
                "[{x}, {y}] is outside the {}x{} screenshot",
                self.shot_width, self.shot_height
            ));
        }
        Ok((
            map_axis(x, self.shot_width, self.logical_width),
            map_axis(y, self.shot_height, self.logical_height),
        ))
    }

    pub fn point_from_logical(&self, x: i32, y: i32) -> (i32, i32) {
        (
            map_axis(x, self.logical_width, self.shot_width),
            map_axis(y, self.logical_height, self.shot_height),
        )
    }

    pub fn delta_to_logical(&self, dx: i32, dy: i32) -> (i32, i32) {
        (
            map_axis(dx, self.shot_width, self.logical_width),
            map_axis(dy, self.shot_height, self.logical_height),
        )
    }
}

pub fn visual_tokens(width: u32, height: u32) -> u32 {
    width.div_ceil(TOKEN_PATCH) * height.div_ceil(TOKEN_PATCH)
}

pub fn fit_screenshot(width: u32, height: u32) -> (u32, u32) {
    if width == 0 || height == 0 || fits(width, height) {
        return (width, height);
    }
    let mut lo = 1u32;
    let mut hi = MAX_LONG_EDGE.min(width.max(height));
    let mut best = scale_long_edge(width, height, 1);
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let (w, h) = scale_long_edge(width, height, mid);
        if fits(w, h) {
            best = (w, h);
            lo = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            hi = mid - 1;
        }
    }
    best
}

fn fits(width: u32, height: u32) -> bool {
    width.max(height) <= MAX_LONG_EDGE && visual_tokens(width, height) <= MAX_VISUAL_TOKENS
}

fn scale_long_edge(width: u32, height: u32, long_edge: u32) -> (u32, u32) {
    if width >= height {
        let h = ((height as u64 * long_edge as u64) / width as u64).max(1) as u32;
        (long_edge.max(1), h)
    } else {
        let w = ((width as u64 * long_edge as u64) / height as u64).max(1) as u32;
        (w, long_edge.max(1))
    }
}

pub fn map_axis(value: i32, src: u32, dst: u32) -> i32 {
    if src == 0 || src == dst {
        return value;
    }
    (value as i64 * dst as i64 / src as i64) as i32
}

/// Map a half-open span from `src` pixels onto `dst` pixels.
pub fn map_span(start: u32, len: u32, src: u32, dst: u32) -> (u32, u32) {
    if src == 0 || dst == 0 || len == 0 {
        return (0, 0);
    }
    if src == dst {
        return (start, len);
    }
    let end = start.saturating_add(len).min(src);
    let d0 = (start as u64 * dst as u64 / src as u64) as u32;
    let mut d1 = (end as u64 * dst as u64 / src as u64) as u32;
    d1 = d1.max(d0.saturating_add(1)).min(dst);
    (d0, d1 - d0)
}

pub fn resize_image(image: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    if width == 0 || height == 0 {
        return RgbaImage::new(1, 1);
    }
    if image.width() == width && image.height() == height {
        return image.clone();
    }
    image::imageops::resize(image, width, height, image::imageops::FilterType::Triangle)
}

pub fn fit_inside(src_w: u32, src_h: u32, box_w: u32, box_h: u32) -> (u32, u32) {
    if src_w == 0 || src_h == 0 || box_w == 0 || box_h == 0 {
        return (1, 1);
    }
    let by_w = box_w as u64 * src_h as u64;
    let by_h = box_h as u64 * src_w as u64;
    if by_w <= by_h {
        let h = ((src_h as u64 * box_w as u64) / src_w as u64).max(1) as u32;
        (box_w, h.min(box_h).max(1))
    } else {
        let w = ((src_w as u64 * box_h as u64) / src_h as u64).max(1) as u32;
        (w.min(box_w).max(1), box_h)
    }
}

pub fn zoom_image(
    device: &RgbaImage,
    grid: Grid,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> Result<RgbaImage, String> {
    if x0 < 0
        || y0 < 0
        || x1 <= x0
        || y1 <= y0
        || x1 > grid.shot_width as i32
        || y1 > grid.shot_height as i32
    {
        return Err(format!(
            "region [{x0}, {y0}, {x1}, {y1}] is outside the {}x{} screenshot",
            grid.shot_width, grid.shot_height
        ));
    }
    let (left, width) = map_span(x0 as u32, (x1 - x0) as u32, grid.shot_width, device.width());
    let (top, height) = map_span(
        y0 as u32,
        (y1 - y0) as u32,
        grid.shot_height,
        device.height(),
    );
    if width == 0
        || height == 0
        || left.saturating_add(width) > device.width()
        || top.saturating_add(height) > device.height()
    {
        return Err("region is outside the screen".into());
    }
    let crop = image::imageops::crop_imm(device, left, top, width, height).to_image();
    let (target_w, target_h) = fit_inside(
        crop.width(),
        crop.height(),
        grid.shot_width,
        grid.shot_height,
    );
    if crop.width() == target_w && crop.height() == target_h {
        return Ok(crop);
    }
    let filter = if crop.width() <= target_w && crop.height() <= target_h {
        image::imageops::FilterType::Nearest
    } else {
        image::imageops::FilterType::Triangle
    };
    Ok(image::imageops::resize(&crop, target_w, target_h, filter))
}

#[cfg(target_os = "macos")]
fn raw_cursor() -> Result<(i32, i32), String> {
    use std::os::raw::c_void;
    #[repr(C)]
    struct CgPoint {
        x: f64,
        y: f64,
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventCreate(source: *const c_void) -> *mut c_void;
        fn CGEventGetLocation(event: *mut c_void) -> CgPoint;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(value: *const c_void);
    }
    unsafe {
        let event = CGEventCreate(std::ptr::null());
        if event.is_null() {
            return Err("could not read the cursor".into());
        }
        let point = CGEventGetLocation(event);
        CFRelease(event);
        Ok((point.x.round() as i32, point.y.round() as i32))
    }
}

#[cfg(not(target_os = "macos"))]
fn raw_cursor() -> Result<(i32, i32), String> {
    Err("cursor position is only implemented on macOS".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn fit_keeps_screens_under_the_limit() {
        for (width, height) in [(1512, 982), (1728, 1117), (2056, 1329), (2560, 1440)] {
            assert_eq!(fit_screenshot(width, height), (width, height));
            assert!(visual_tokens(width, height) <= MAX_VISUAL_TOKENS);
        }
    }

    #[test]
    fn fit_shrinks_4k_without_stretching() {
        let (width, height) = fit_screenshot(3840, 2160);
        assert_eq!((width, height), (2576, 1449));
        assert!(width <= MAX_LONG_EDGE && height <= MAX_LONG_EDGE);
        assert!(visual_tokens(width, height) <= MAX_VISUAL_TOKENS);
        let aspect = width as f64 / height as f64;
        assert!((aspect - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn point_round_trip_on_a_shrunk_grid() {
        let grid = Grid::from_logical(3840, 2160);
        let x = grid.shot_width as i32 / 2;
        let (logical_x, _) = grid.point_to_logical(x, 0).unwrap();
        let (shot_x, _) = grid.point_from_logical(logical_x, 0);
        assert_eq!(shot_x, x);
        assert!(grid.point_to_logical(grid.shot_width as i32, 0).is_err());
    }

    #[test]
    fn zoom_keeps_device_pixels_the_full_shot_blends_away() {
        let mut device = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 255, 255]));
        device.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        device.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
        let grid = Grid::from_logical(4, 4);
        let zoomed = zoom_image(&device, grid, 0, 0, 1, 1).unwrap();
        assert!(has_pure(&zoomed, [255, 0, 0]));
        assert!(has_pure(&zoomed, [0, 255, 0]));
        let shot = resize_image(&device, grid.shot_width, grid.shot_height);
        let shot_has_both = has_pure(&shot, [255, 0, 0]) && has_pure(&shot, [0, 255, 0]);
        assert!(!shot_has_both);
    }

    fn has_pure(image: &RgbaImage, rgb: [u8; 3]) -> bool {
        image.pixels().any(|pixel| pixel.0[0..3] == rgb)
    }
}
