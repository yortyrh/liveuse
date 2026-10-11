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

pub fn capture_primary() -> Result<RgbaImage, String> {
    let monitors = xcap::Monitor::all().map_err(|err| err.to_string())?;
    let primary = monitors
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| "no primary display".to_string())?;
    let logical_w = primary.width().map_err(|err| err.to_string())?;
    let logical_h = primary.height().map_err(|err| err.to_string())?;
    let captured = primary.capture_image().map_err(|err| err.to_string())?;
    if captured.width() == logical_w && captured.height() == logical_h {
        return Ok(captured);
    }
    Ok(image::imageops::resize(
        &captured,
        logical_w,
        logical_h,
        image::imageops::FilterType::Triangle,
    ))
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
