use super::{audio::AudioManager, font::FontCache, image::ImageCache, model::Model};

use super::draw::FRAME_DIMS_U32;

const PNG_COLOR: png::ColorType = png::ColorType::Rgba;
const PNG_DEPTH: png::BitDepth = png::BitDepth::Eight;

pub struct Manager {
    audio_manager: AudioManager,
    font_cache: FontCache,
    image_cache: ImageCache,
    draw_context: super::draw::Context,
}

pub trait PngFrameWriter {
    fn write_frame(&mut self, frame: &PngFrame);
}

pub struct PngFrame {
    pub width: &'static u32,
    pub height: &'static u32,
    pub color: &'static png::ColorType,
    pub depth: &'static png::BitDepth,
    pub png_data: Vec<u8>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            audio_manager: Default::default(),
            font_cache: Default::default(),
            image_cache: Default::default(),
            draw_context: super::draw::Context::new(),
        }
    }

    pub fn setup(&mut self) {
        self.audio_manager.setup();
        self.font_cache.setup();
        self.image_cache.setup();
    }

    pub fn write_frame<W: PngFrameWriter>(&mut self, model: &Model, writer: &mut W) {
        let frame = PngFrame::new(self.make_png_data(&model));
        writer.write_frame(&frame);
    }

    fn make_png_data(&mut self, model: &Model) -> Vec<u8> {
        let images = self.image_cache.get_images();
        let fonts = self.font_cache.get_fonts();
        self.draw_context.make_png_data(model, images, fonts)
    }
}

impl PngFrame {
    fn new(png_data: Vec<u8>) -> Self {
        Self { width: &FRAME_DIMS_U32.0, height: &FRAME_DIMS_U32.1, color: &PNG_COLOR, depth: &PNG_DEPTH, png_data }
    }
}
