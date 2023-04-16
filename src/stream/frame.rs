use std::{fs::File, io::Write, path::PathBuf};

use crate::error::Result;

use super::{draw::FRAME_DIMS_U32, font::Fonts, image::Images, model::Model};

const PNG_COLOR: png::ColorType = png::ColorType::Rgba;
const PNG_DEPTH: png::BitDepth = png::BitDepth::Eight;

pub struct FrameManager {
    draw_context: super::draw::Context,
    current_frame: PngFrame,
    frame_update_required: bool,
    video_fifo: File,
}

pub struct PngFrame {
    pub width: &'static u32,
    pub height: &'static u32,
    pub color: &'static png::ColorType,
    pub depth: &'static png::BitDepth,
    pub data: Vec<u8>,
}

impl FrameManager {
    pub fn new(video_fifo: PathBuf) -> Result<Self> {
        let video_fifo = File::create(&video_fifo)?;

        let frame_manager = Self {
            draw_context: super::draw::Context::new(),
            current_frame: Default::default(),
            frame_update_required: true,
            video_fifo,
        };

        Ok(frame_manager)
    }

    pub fn needs_update(&self) -> bool {
        self.frame_update_required
    }

    pub fn set_needs_update(&mut self) {
        self.frame_update_required = true;
    }

    pub fn update_frame(&mut self, model: &Model, images: &Images, fonts: &Fonts) {
        if self.frame_update_required {
            let png_data = self.draw_context.make_png_data(&model, images, fonts);
            self.current_frame = PngFrame::new(png_data);
            self.frame_update_required = false;
        }
    }

    pub fn write_frame(&mut self) -> std::io::Result<()> {
        Ok(self.video_fifo.write_all(&self.current_frame.data)?)
    }
}

impl Default for PngFrame {
    fn default() -> Self {
        let byte_count = (4 * FRAME_DIMS_U32.0 * FRAME_DIMS_U32.1) as usize;
        Self::new(vec![0; byte_count])
    }
}

impl PngFrame {
    fn new(data: Vec<u8>) -> Self {
        Self {
            width: &FRAME_DIMS_U32.0,
            height: &FRAME_DIMS_U32.1,
            color: &PNG_COLOR,
            depth: &PNG_DEPTH,
            data,
        }
    }
}
