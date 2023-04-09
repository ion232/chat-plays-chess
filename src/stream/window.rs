use super::draw::FRAME_DIMS_U32;
use super::manager::PngFrame;
use super::manager::PngFrameWriter;

/// For testing purposes mostly.
/// Although the window itself could be streamed using OBS, etc.
pub struct Window {
    window: minifb::Window,
}

impl Window {
    pub fn new() -> Self {
        let name = "ChatPlaysChess";
        let width = FRAME_DIMS_U32.0 as usize;
        let height = FRAME_DIMS_U32.1 as usize;
        let options = minifb::WindowOptions::default();
        let window = minifb::Window::new(name, width, height, options).unwrap();

        Self { window }
    }
}

impl PngFrameWriter for Window {
    fn write_frame(&mut self, frame: &PngFrame) {
        let width = *frame.width as usize;
        let height = *frame.height as usize;
        let data: Vec<u32> = frame
            .png_data
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        self.window.update_with_buffer(&data, width, height).unwrap();
    }
}
