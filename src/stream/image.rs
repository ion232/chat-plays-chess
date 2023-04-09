use std::collections::HashMap;
use std::fs::File;

use raqote::Image;

#[derive(Default)]
pub struct ImageCache {
    images: HashMap<String, ImageData>,
}

pub struct ImageData {
    width: i32,
    height: i32,
    buffer: Vec<u32>,
}

impl ImageData {
    fn new(width: i32, height: i32, buffer: Vec<u32>) -> Self {
        Self { width, height, buffer }
    }

    fn as_image(&self) -> Image {
        Image { width: self.width, height: self.height, data: &self.buffer[..] }
    }
}

impl ImageCache {
    pub fn setup(&mut self) {
        self.load_all_images();
    }

    pub fn get_images(&mut self) -> Images {
        Images {
            board: Board { dark: self.get_image("background/dark"), light: self.get_image("background/light") },
            black_pieces: Pieces {
                king: self.get_image("pieces/black_king"),
                queen: self.get_image("pieces/black_queen"),
                rook: self.get_image("pieces/black_rook"),
                bishop: self.get_image("pieces/black_bishop"),
                knight: self.get_image("pieces/black_knight"),
                pawn: self.get_image("pieces/black_pawn"),
            },
            white_pieces: Pieces {
                king: self.get_image("pieces/white_king"),
                queen: self.get_image("pieces/white_queen"),
                rook: self.get_image("pieces/white_rook"),
                bishop: self.get_image("pieces/white_bishop"),
                knight: self.get_image("pieces/white_knight"),
                pawn: self.get_image("pieces/white_pawn"),
            },
        }
    }

    fn get_image(&self, k: &str) -> Image {
        self.images.get(k).unwrap().as_image()
    }

    fn load_all_images(&mut self) {
        self.images.clear();

        self.load_image_data("background/dark");
        self.load_image_data("background/light");

        self.load_image_data("pieces/black_king");
        self.load_image_data("pieces/black_queen");
        self.load_image_data("pieces/black_rook");
        self.load_image_data("pieces/black_bishop");
        self.load_image_data("pieces/black_knight");
        self.load_image_data("pieces/black_pawn");

        self.load_image_data("pieces/white_king");
        self.load_image_data("pieces/white_queen");
        self.load_image_data("pieces/white_rook");
        self.load_image_data("pieces/white_bishop");
        self.load_image_data("pieces/white_knight");
        self.load_image_data("pieces/white_pawn");
    }

    fn load_image_data(&mut self, name: &str) {
        let path = std::fmt::format(format_args!("assets/images/{}.png", &name));
        let image_data = load_png(&path);
        self.images.insert(name.to_string(), image_data);
    }
}

fn load_png(path: &str) -> ImageData {
    let image_file = File::open(path).unwrap();
    let png_decoder = png::Decoder::new(image_file);
    let mut png_reader = png_decoder.read_info().unwrap();

    let mut image_bytes = vec![0; png_reader.output_buffer_size()];
    let png_info = png_reader.next_frame(&mut image_bytes).unwrap();

    let pixel_count = png_info.width as usize * png_info.height as usize;
    let chunk_size = png_info.color_type.samples();

    let mut image_buffer = Vec::<u32>::with_capacity(pixel_count);
    let image_bytes = image_bytes.chunks(chunk_size).filter(|s| s.len() == chunk_size);

    for bytes in image_bytes {
        let rgba = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        image_buffer.push(rgba);
    }

    ImageData::new(png_info.width as i32, png_info.height as i32, image_buffer)
}

pub struct Images<'a> {
    pub board: Board<'a>,
    pub black_pieces: Pieces<'a>,
    pub white_pieces: Pieces<'a>,
}

pub struct Board<'a> {
    pub dark: Image<'a>,
    pub light: Image<'a>,
}

pub struct Pieces<'a> {
    pub king: Image<'a>,
    pub queen: Image<'a>,
    pub rook: Image<'a>,
    pub bishop: Image<'a>,
    pub knight: Image<'a>,
    pub pawn: Image<'a>,
}
