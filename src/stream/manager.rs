use std::path::PathBuf;
use std::time::Duration;

use crate::engine::events::stream::{Action, Event, EventReceiver, GameUpdate, Notification};
use crate::error::Result;

use super::frame::FrameManager;
use super::model::Side;
use super::{audio::AudioManager, font::FontCache, image::ImageCache, model::Model};

const FRAME_TIME: Duration = Duration::from_millis((1000.0 / 30.0) as u64);

pub struct Manager {
    audio_manager: AudioManager,
    font_cache: FontCache,
    image_cache: ImageCache,
    frame_manager: FrameManager,
    model: Model,
    stream_events: EventReceiver,
    is_running: bool,
}

impl Manager {
    pub fn new(stream_events: EventReceiver, video_fifo: PathBuf) -> Result<Self> {
        let manager = Self {
            audio_manager: Default::default(),
            font_cache: Default::default(),
            image_cache: Default::default(),
            frame_manager: FrameManager::new(video_fifo)?,
            model: Default::default(),
            stream_events,
            is_running: false,
        };

        Ok(manager)
    }

    pub fn setup(&mut self) {
        _ = self.audio_manager.setup();
        self.font_cache.setup();
        self.image_cache.setup();
    }

    /// Needs to be run on a separate thread.
    pub fn run(&mut self) {
        self.is_running = true;

        let mut now = std::time::Instant::now();

        while self.is_running {
            std::thread::sleep(Duration::from_millis(1));

            self.process_events();

            if now.elapsed() < FRAME_TIME {
                continue;
            } else {
                now = std::time::Instant::now();
            }

            if self.frame_manager.needs_update() {
                let images = self.image_cache.images();
                let fonts = self.font_cache.fonts();
                self.frame_manager.update_frame(&self.model, &images, &fonts);
            }

            if let Err(error) = self.frame_manager.write_frame() {
                log::error!("Failed to write frame: {}", error);

                if let std::io::ErrorKind::BrokenPipe = error.kind() {
                    self.is_running = false;
                }
            }
        }
    }

    fn process_events(&mut self) {
        while !self.stream_events.is_empty() {
            if let Some(event) = self.stream_events.recv().ok() {
                self.process_event(event);
            };
        }
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::Action(action) => self.process_action(action),
            Event::Notification(notification) => self.process_notification(notification),
        }
    }

    fn process_action(&mut self, action: Action) {
        match action {
            Action::PlayClip { clip } => self.audio_manager.play_clip(clip),
            Action::Shutdown => self.is_running = false,
        }
    }

    fn process_notification(&mut self, notification: Notification) {
        match notification {
            Notification::ActiveGame { game } => self.model.update_from_game(game),
            Notification::ChatCommand { command } => self.model.chat_commands.push(command),
            Notification::Notice { notice } => self.model.notice = notice,
            Notification::State { state } => self.model.state = state,
            Notification::Settings { settings } => self.model.settings = settings,
            Notification::GameVotes { votes } => self.model.game_votes = votes,
            Notification::GameUpdate(game_update) => match game_update {
                GameUpdate::Board { board } => self.model.board = board,
                GameUpdate::MoveHistory { moves } => self.model.move_history = moves,
                GameUpdate::Timer { side, timer } => match side {
                    Side::Ours => self.model.us.timer = timer,
                    Side::Theirs => self.model.opponent.timer = timer,
                },
            },
        }

        self.frame_manager.set_needs_update();
    }
}
