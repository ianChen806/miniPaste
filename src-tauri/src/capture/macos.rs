use super::{Capture, CaptureError, CaptureFrame};

pub struct MacosCapture;

impl MacosCapture {
    pub fn new() -> Self {
        Self
    }
}

impl Capture for MacosCapture {
    fn virtual_desktop(&self) -> Result<CaptureFrame, CaptureError> {
        unimplemented!("non-Windows capture support deferred")
    }
    // `crop` uses the trait default which delegates to `super::crop_png`.
}
