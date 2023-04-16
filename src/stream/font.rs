use std::{collections::HashMap, fs::File, io::Read};

use fontdue::{Font, FontSettings};

#[derive(Default)]
pub struct FontCache {
    fonts: HashMap<String, Font>,
}

pub struct Fonts {
    pub gb: Font,
    pub retro: Font,
}

impl FontCache {
    pub fn setup(&mut self) {
        self.load_all_fonts();
    }

    pub fn fonts(&self) -> Fonts {
        Fonts { gb: self.get_font("PokemonGB"), retro: self.get_font("VCR_OSD_MONO") }
    }

    fn get_font(&self, k: &str) -> Font {
        self.fonts.get(k).unwrap().clone()
    }

    fn load_all_fonts(&mut self) {
        self.fonts.clear();
        self.load_ttf_file("PokemonGB");
        self.load_ttf_file("VCR_OSD_MONO");
    }

    fn load_ttf_file(&mut self, name: &str) {
        let path = format!("assets/fonts/{}.ttf", &name);
        let font_file = File::open(path).unwrap();
        let font_data: Vec<u8> = font_file.bytes().map(|x| x.unwrap()).collect();

        let font = Font::from_bytes(&font_data[..], FontSettings::default()).unwrap();
        self.fonts.insert(name.to_string(), font);
    }
}
