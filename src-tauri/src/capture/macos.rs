use super::{Capture, CaptureError, CaptureFrame, Rect};

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

    fn crop(&self, _frame: &CaptureFrame, _rect: Rect) -> Result<Vec<u8>, CaptureError> {
        unimplemented!("non-Windows capture support deferred")
    }
}
