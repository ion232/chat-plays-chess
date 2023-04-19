/// This file is full of hardcoded fudge-factors, but it serves it's purpose.
/// It would be reasonably simple to abstract this into something cleaner, but would take more time.
use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle, VerticalAlign};
use raqote::{
    Color, DrawOptions, DrawTarget, Gradient, GradientStop, Image, LineCap, LineJoin, PathBuilder,
    Point, SolidSource, Source, Spread, StrokeStyle,
};

use fontdue::Font;

use crate::engine::votes::settings::Settings;

use super::font::Fonts;
use super::image::Images;
use super::model::{Command, GameVotes, Model, Notice, Player, State, Title};

pub const FRAME_DIMS_U32: (u32, u32) = (1920, 1080);
pub const FRAME_DIMS_F32: (f32, f32) = (1920.0, 1080.0);
pub const FRAME_PIXEL_COUNT: usize = FRAME_DIMS_U32.0 as usize * FRAME_DIMS_U32.1 as usize;

// Left column.

const NOTICE_ORIGIN: (f32, f32) = (0.0, 0.0);
const NOTICE_DIMS: (f32, f32) = (620.0, 200.0);

const CURRENT_STATE_ORIGIN: (f32, f32) = (NOTICE_ORIGIN.0, NOTICE_ORIGIN.1 + NOTICE_DIMS.1);
const CURRENT_STATE_DIMS: (f32, f32) = (NOTICE_DIMS.0, 100.0);

const SETTINGS_ORIGIN: (f32, f32) =
    (NOTICE_ORIGIN.0, CURRENT_STATE_ORIGIN.1 + CURRENT_STATE_DIMS.1);
const SETTINGS_DIMS: (f32, f32) = (NOTICE_DIMS.0, 240.0);

const MOVE_HISTORY_ORIGIN: (f32, f32) = (NOTICE_ORIGIN.0, SETTINGS_ORIGIN.1 + SETTINGS_DIMS.1);
const MOVE_HISTORY_DIMS: (f32, f32) = (NOTICE_DIMS.0, 540.0);

// Middle column.

const TITLE_ORIGIN: (f32, f32) = (NOTICE_ORIGIN.0 + NOTICE_DIMS.0, 0.0);
const TITLE_DIMS: (f32, f32) = (680.0, 200.0);

const PLAYER_DIMS: (f32, f32) = (TITLE_DIMS.0, 100.0);

const OPPONENT_ORIGIN: (f32, f32) = (TITLE_ORIGIN.0, TITLE_ORIGIN.1 + TITLE_DIMS.1);
const OPPONENT_DIMS: (f32, f32) = PLAYER_DIMS;

const BOARD_ORIGIN: (f32, f32) = (TITLE_ORIGIN.0, OPPONENT_ORIGIN.1 + OPPONENT_DIMS.1);
const BOARD_DIMS: (f32, f32) = (TITLE_DIMS.0, TITLE_DIMS.0);
const SQUARE_DIMS: (f32, f32) = (BOARD_DIMS.0 / 8.0, BOARD_DIMS.1 / 8.0);

const USER_ORIGIN: (f32, f32) = (TITLE_ORIGIN.0, BOARD_ORIGIN.1 + BOARD_DIMS.1);
const _USER_DIMS: (f32, f32) = PLAYER_DIMS;

// Right column.

const GAME_VOTES_ORIGIN: (f32, f32) = (TITLE_ORIGIN.0 + TITLE_DIMS.0, 0.0);
const GAME_VOTES_DIMS: (f32, f32) = (620.0, FRAME_DIMS_F32.1 / 2.0);

const COMMANDS_ORIGIN: (f32, f32) = (GAME_VOTES_ORIGIN.0, GAME_VOTES_ORIGIN.1 + GAME_VOTES_DIMS.1);
const COMMANDS_DIMS: (f32, f32) = (1200.0, FRAME_DIMS_F32.1 / 2.0);

// Draw properties.

const BORDER_STROKE_WIDTH: f32 = 4.0;

pub struct Context {
    target: DrawTarget,
    sources: Sources,
    strokes: StrokeStyles,
}

struct Sources {
    box_border: SolidSource,
    black: SolidSource,
    white: SolidSource,
}

struct StrokeStyles {
    border: StrokeStyle,
}

impl Context {
    pub fn new() -> Self {
        let (width, height) = (FRAME_DIMS_U32.0 as i32, FRAME_DIMS_U32.1 as i32);
        let border_stroke = StrokeStyle {
            width: BORDER_STROKE_WIDTH,
            cap: LineCap::Square,
            join: LineJoin::Miter,
            miter_limit: 2.0,
            dash_array: vec![],
            dash_offset: 0.0,
        };
        Self {
            target: DrawTarget::new(width, height),
            sources: Sources {
                box_border: SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0),
                black: SolidSource::from_unpremultiplied_argb(0xff, 0, 0, 0),
                white: SolidSource::from_unpremultiplied_argb(0xff, 0xff, 0xff, 0xff),
            },
            strokes: StrokeStyles { border: border_stroke },
        }
    }

    pub fn make_png_data(&mut self, model: &Model, images: &Images, fonts: &Fonts) -> Vec<u8> {
        self.clear();
        self.draw_elements(&model, &images, &fonts);
        self.as_png_data()
    }

    fn clear(&mut self) {
        self.target.clear(self.sources.white);
    }

    fn draw_elements(&mut self, model: &Model, images: &Images, fonts: &Fonts) {
        self.draw_notice(&model.notice, &fonts);
        self.draw_current_state(&model.state, &fonts);
        self.draw_settings(&model.settings, &fonts);
        self.draw_move_history(&model.move_history, &fonts);

        self.draw_title(&model.title, &fonts);
        self.draw_opponent_bar(&model.opponent, &fonts);
        self.draw_chess_board(&images.board.dark, &images.board.light);
        self.draw_chess_pieces(&model.us, &model.board, images);
        self.draw_our_bar(&model.us, &fonts);

        self.draw_game_votes(&model.game_votes, &fonts);
        self.draw_chat_commands(&model.chat_commands, &fonts);
    }

    fn as_png_data(&mut self) -> Vec<u8> {
        // To avoid branching on every one of the ~120 million pixels processed per second.
        let mut norm = [0u32; 256];
        norm[0] = 1;

        let target_data = self.target.get_data();
        let mut png_data = Vec::with_capacity(target_data.len() * 4);

        for pixel in target_data {
            let a = (pixel >> 24) & 0xffu32;
            let mut r = (pixel >> 16) & 0xffu32;
            let mut g = (pixel >> 8) & 0xffu32;
            let mut b = (pixel >> 0) & 0xffu32;

            let f = a + norm[a as usize];
            r = r * 255u32 / f;
            g = g * 255u32 / f;
            b = b * 255u32 / f;

            png_data.push(r as u8);
            png_data.push(g as u8);
            png_data.push(b as u8);
            png_data.push(a as u8);
        }

        png_data
    }

    fn draw_notice(&mut self, notice: &Notice, fonts: &Fonts) {
        let (x, y) = NOTICE_ORIGIN;
        let (width, height) = NOTICE_DIMS;

        self.draw_box(x, y, width, height);
        self.draw_lines(x + 24.0, y + 24.0, &fonts.retro, 32.0, &notice.lines)
    }

    fn draw_current_state(&mut self, state: &State, fonts: &Fonts) {
        let (x, y) = CURRENT_STATE_ORIGIN;
        let (width, height) = CURRENT_STATE_DIMS;
        let text = state.to_string();
        self.draw_box(x, y, width, height);

        let black = self.sources.black.clone();
        let green = SolidSource::from_unpremultiplied_argb(0xff, 73, 133, 53);
        let light_red = SolidSource::from_unpremultiplied_argb(0xff, 201, 34, 22);
        let brown = SolidSource::from_unpremultiplied_argb(0xff, 128, 72, 60);

        let color = match state {
            State::ChallengingUser { .. } => brown,
            State::OurTurn => green,
            State::TheirTurn => light_red,
            State::GameFinished => green,
            State::Unknown => black,
        };

        self.draw_coloured_text(x + 24.0, y + 32.0, &fonts.retro, 32.0, &text, color);
    }

    fn draw_settings(&mut self, settings: &Settings, fonts: &Fonts) {
        let (x, y) = SETTINGS_ORIGIN;
        let (width, height) = SETTINGS_DIMS;
        let lines = settings.lines();
        self.draw_box(x, y, width, height);
        self.draw_lines(x + 24.0, y + 32.0, &fonts.retro, 40.0, &lines)
    }

    fn draw_move_history(&mut self, move_history: &Vec<String>, fonts: &Fonts) {
        let (x, y) = MOVE_HISTORY_ORIGIN;
        let (width, height) = MOVE_HISTORY_DIMS;
        let mut move_history = move_history.clone();
        if move_history.len() % 2 != 0 {
            move_history.push("".to_string());
        }
        let mut pairs: Vec<(String, String)> =
            move_history.chunks(2).map(|pair| (pair[0].to_string(), pair[1].to_string())).collect();
        let pairs: Vec<(String, String)> = pairs.split_off(pairs.len().saturating_sub(28));
        let first_column: Vec<String> = pairs
            .iter()
            .enumerate()
            .take(14)
            .map(|(index, (m1, m2))| format!("{}: {} {}", index + 1, m1, m2))
            .collect();
        let second_column: Vec<String> = pairs
            .iter()
            .enumerate()
            .skip(14)
            .map(|(index, (m1, m2))| format!("{}: {} {}", index + 1, m1, m2))
            .collect();
        self.draw_box(x, y, width, height);
        self.draw_lines(x + 12.0, y + 12.0, &fonts.retro, 30.0, &first_column);
        self.draw_lines(x + 12.0, y + 12.0, &fonts.retro, 30.0, &second_column);
    }

    fn draw_title(&mut self, title: &Title, fonts: &Fonts) {
        let (x, y) = TITLE_ORIGIN;
        let (width, height) = TITLE_DIMS;

        self.draw_box(x, y, width, height);
        self.draw_text(x + 12.0, y + 36.0, &fonts.retro, 64.0, &title.to_string());
        self.draw_text(x + 12.0, y + 148.0, &fonts.retro, 40.0, &title.url.to_string());
    }

    fn draw_opponent_bar(&mut self, opponent: &Player, fonts: &Fonts) {
        let (x, y) = OPPONENT_ORIGIN;
        self.draw_player_bar(x, y, opponent, &fonts.retro);
    }

    fn draw_chess_board(&mut self, dark: &Image, light: &Image) {
        for x in 0..8 {
            for y in 0..8 {
                self.draw_chess_square(x, y, dark, light);
            }
        }
    }

    fn draw_chess_pieces(&mut self, us: &Player, board: &chess::Board, images: &Images) {
        let (file_offset, rank_offset) =
            if chess::Color::Black == us.color { (7, 0) } else { (0, 7) };

        for square in chess::ALL_SQUARES {
            let file = (file_offset - square.get_file().to_index() as i32).abs();
            let rank = (rank_offset - square.get_rank().to_index() as i32).abs();

            if let Some(piece) = board.piece_on(square) {
                if let Some(color) = board.color_on(square) {
                    let pieces = match color {
                        chess::Color::White => &images.white_pieces,
                        chess::Color::Black => &images.black_pieces,
                    };
                    let image = match piece {
                        chess::Piece::Pawn => pieces.pawn,
                        chess::Piece::Knight => pieces.knight,
                        chess::Piece::Bishop => pieces.bishop,
                        chess::Piece::Rook => pieces.rook,
                        chess::Piece::Queen => pieces.queen,
                        chess::Piece::King => pieces.king,
                    };
                    let x = 6.0 + BOARD_ORIGIN.0 + (SQUARE_DIMS.0 * (file as f32 + 0.0)) as f32;
                    let mut y = 6.0 + BOARD_ORIGIN.1 + (SQUARE_DIMS.1 * (rank as f32 + 0.0)) as f32;

                    if piece == chess::Piece::Pawn {
                        y -= 4.0;
                    }

                    self.draw_image(x, y, SQUARE_DIMS.0 - 12.0, SQUARE_DIMS.1 - 12.0, &image);
                }
            }
        }
    }

    fn draw_chess_square(&mut self, x: i32, y: i32, dark: &Image, light: &Image) {
        let x_even = x % 2 == 0;
        let y_even = y % 2 == 0;
        let is_light = (x_even && y_even) || (!x_even && !y_even);
        let image = if is_light { light } else { dark };

        let offset = BORDER_STROKE_WIDTH / 2.0;
        let x = offset + BOARD_ORIGIN.0 + (x as f32 * SQUARE_DIMS.0);
        let y = offset + BOARD_ORIGIN.1 + (y as f32 * SQUARE_DIMS.1);

        self.draw_image(x, y, SQUARE_DIMS.0, SQUARE_DIMS.1, &image);
    }

    fn draw_our_bar(&mut self, us: &Player, fonts: &Fonts) {
        let (x, y) = USER_ORIGIN;
        self.draw_player_bar(x, y, us, &fonts.retro);
    }

    fn draw_game_votes(&mut self, game_votes: &GameVotes, fonts: &Fonts) {
        let (x, y) = GAME_VOTES_ORIGIN;
        let (width, height) = GAME_VOTES_DIMS;
        let lines = game_votes.lines();
        self.draw_box(x, y, width, height);
        self.draw_lines(x + 12.0, y + 12.0, &fonts.retro, 42.0, &lines);
    }

    fn draw_chat_commands(&mut self, chat_commands: &Vec<Command>, fonts: &Fonts) {
        let (x, y) = COMMANDS_ORIGIN;
        let (width, height) = COMMANDS_DIMS;
        let lines = chat_commands.into_iter().rev().take(14).map(|c| c.to_string()).collect();
        self.draw_box(x, y, width, height);
        self.draw_text(x + 12.0, y + 12.0, &fonts.retro, 42.0, "Chat commands:");
        self.draw_lines(x + 12.0, y + 64.0, &fonts.retro, 32.0, &lines);
    }

    fn draw_player_bar(&mut self, x: f32, y: f32, player: &Player, font: &Font) {
        let (width, height) = PLAYER_DIMS;
        self.draw_box(x, y, width, height);
        self.draw_text(x + 12.0, y + 12.0, font, 40.0, &player.to_string());
    }

    // Utility functions.

    fn draw_box(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let style = &self.strokes.border;
        let sw = style.width / 2.0;

        let mut path_builder = PathBuilder::new();
        path_builder.rect(x + sw, y + sw, width - sw, height - sw);

        let path = path_builder.finish();
        let options = DrawOptions::new();
        let stops = vec![
            GradientStop { position: 0.0, color: Color::new(0xff, 0xaa, 0xaa, 0xaa) },
            GradientStop { position: 1.0, color: Color::new(0xff, 0xff, 0xff, 0xff) },
        ];
        let fill_source = Source::new_linear_gradient(
            Gradient { stops },
            Point::new(x, y),
            Point::new(x + width, y + height),
            Spread::Pad,
        );
        let border_source = Source::Solid(self.sources.box_border);

        self.target.fill(&path, &fill_source, &options);
        self.target.stroke(&path, &border_source, &style, &options)
    }

    fn draw_image(&mut self, x: f32, y: f32, width: f32, height: f32, image: &Image) {
        let options = DrawOptions::new();
        self.target.draw_image_with_size_at(width, height, x, y, image, &options);
    }

    fn draw_lines(&mut self, x: f32, y: f32, font: &Font, size: f32, lines: &Vec<String>) {
        for (i, line) in lines.iter().enumerate() {
            self.draw_text(x, y + (i as f32 * size) + 4.0, font, size, line);
        }
    }

    fn draw_text(&mut self, x: f32, y: f32, font: &Font, size: f32, text: &str) {
        self.draw_coloured_text(x, y, font, size, text, self.sources.black.clone())
    }

    fn draw_coloured_text(
        &mut self,
        x: f32,
        y: f32,
        font: &Font,
        size: f32,
        text: &str,
        source: SolidSource,
    ) {
        let options = DrawOptions::new();

        // Sourced and edited from: https://github.com/l4l/yofi/blob/53863d39b5c2c5709df280fba1da7a80dd924492/src/font/fdue.rs#L172-L227
        // TODO: Figure out how much space is needed for the buffer.
        let mut buffer = vec![0; 256 * 256];
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);

        layout.reset(&LayoutSettings {
            x,
            y,
            max_height: Some(size),
            vertical_align: VerticalAlign::Bottom,
            ..LayoutSettings::default()
        });

        layout.append(&[font], &TextStyle::new(text, size, 0));

        for g in layout.glyphs().iter() {
            let (_, b) = font.rasterize_config(g.key);

            assert!(g.width * g.height <= buffer.capacity());
            let width = g.width as i32;
            let height = g.height as i32;

            for (i, x) in b.into_iter().enumerate() {
                let src = SolidSource::from_unpremultiplied_argb(
                    (u32::from(x) * u32::from(source.a) / 255) as u8,
                    source.r,
                    source.g,
                    source.b,
                );
                buffer[i] = (u32::from(src.a) << 24)
                    | (u32::from(src.r) << 16)
                    | (u32::from(src.g) << 8)
                    | u32::from(src.b);
            }

            let image = raqote::Image { width, height, data: &buffer[..] };

            self.target.draw_image_with_size_at(
                g.width as f32,
                g.height as f32,
                g.x,
                g.y,
                &image,
                &options,
            );
        }
    }
}
