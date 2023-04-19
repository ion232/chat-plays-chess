use chat_plays_chess::config;
use chat_plays_chess::stream::draw::{FRAME_DIMS_U32, FRAME_PIXEL_COUNT};

use std::{fs::File, path::PathBuf};

const BYTES_IN_PIXEL: usize = 4;

struct Window {
    width: usize,
    height: usize,
    window: minifb::Window,
    frame_buffer: Vec<u8>,
    video_fifo: File,
}

impl Window {
    pub fn new(name: &str, video_fifo: PathBuf) -> Self {
        let width = FRAME_DIMS_U32.0 as usize;
        let height = FRAME_DIMS_U32.1 as usize;
        let options = minifb::WindowOptions::default();

        let window = minifb::Window::new(name, width, height, options).unwrap();
        let frame_buffer = vec![0; BYTES_IN_PIXEL * FRAME_PIXEL_COUNT];
        let video_fifo = File::open(video_fifo).unwrap();

        Self { width, height, window, frame_buffer, video_fifo }
    }
}

impl Window {
    fn run(&mut self) -> Result<(), png::DecodingError> {
        loop {
            let mut frame_buffer = self.read_frame()?;
            _ = self.window.update_with_buffer(&mut frame_buffer, self.width, self.height);
        }
    }

    fn read_frame(&mut self) -> Result<Vec<u32>, png::DecodingError> {
        let decoder = png::Decoder::new(self.video_fifo.try_clone().unwrap());

        let mut reader = decoder.read_info()?;
        _ = reader.next_frame(&mut self.frame_buffer)?;

        let frame_buffer: Vec<u32> = self
            .frame_buffer
            .chunks_exact(BYTES_IN_PIXEL)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        Ok(frame_buffer)
    }
}

fn main() {
    let Ok(config) = config::load_config() else {
        println!("Window: Failed to load config!");
        return;
    };

    let mut window = Window::new("ChatPlaysChess", config.livestream.video.fifo.into());
    window.run();
}
