use super::{Capture, CaptureError, CaptureFrame, Rect};

pub struct WindowsCapture;

impl WindowsCapture {
    pub fn new() -> Self {
        Self
    }
}

impl Capture for WindowsCapture {
    fn virtual_desktop(&self) -> Result<CaptureFrame, CaptureError> {
        Err(CaptureError::Backend("not yet implemented".into()))
    }

    fn crop(&self, _frame: &CaptureFrame, _rect: Rect) -> Result<Vec<u8>, CaptureError> {
        Err(CaptureError::Backend("not yet implemented".into()))
    }
}
