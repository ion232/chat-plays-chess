use std::time::Duration;

use crossbeam::select;
use crossbeam_channel::{Receiver, Sender};

use crate::error::Result;

use crate::lichess;
use crate::lichess::events::Event as LichessEvent;
use crate::lichess::events::EventManager as LichessEventManager;

use crate::twitch;
use crate::twitch::events::Event as TwitchEvent;
use crate::twitch::events::EventManager as TwitchEventManager;

pub struct EventManager {
    lichess: EventSource<LichessEvent, LichessEventManager>,
    twitch: EventSource<TwitchEvent, TwitchEventManager>,
}

struct EventSource<E, M> {
    pub(crate) event_manager: M,
    pub(crate) receiver: Receiver<Result<E>>,
    pub(crate) sender: Sender<Result<E>>,
}

pub enum Event {
    Lichess(LichessEvent),
    Twitch(TwitchEvent),
}

impl EventManager {
    pub fn new(lichess_context: lichess::Context, twitch_context: twitch::Context) -> Self {
        Self {
            lichess: EventSource::new(LichessEventManager::new(lichess_context)),
            twitch: EventSource::new(TwitchEventManager::new(twitch_context)),
        }
    }

    pub async fn subscribe_to_all(&mut self) -> Result<()> {
        self.lichess.event_manager.stream_account(self.lichess.sender.clone()).await?;
        // self.twitch.event_manager.stream_twitch_irc_events(self.twitch.sender.clone()).await?;

        Ok(())
    }

    pub async fn stream_game(&mut self, game_id: &str) -> Result<()> {
        self.lichess.event_manager.stream_game(self.lichess.sender.clone(), game_id).await
    }

    pub async fn finish_streaming_game(&mut self, game_id: &str) {
        self.lichess.event_manager.finish_streaming_game(game_id).await
    }

    pub fn next_event(&self) -> Result<Option<Event>> {
        // I think it's possible to refactor this with select!.

        if !self.lichess.receiver.is_empty() {
            if let Ok(event) = self.lichess.receiver.recv() {
                return Ok(Some(Event::from(event?)));
            }
        }

        if !self.twitch.receiver.is_empty() {
            if let Ok(event) = self.twitch.receiver.recv() {
                return Ok(Some(Event::from(event?)));
            }
        }

        Ok(None)
    }
}

impl<E, M> EventSource<E, M> {
    pub fn new(event_manager: M) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { event_manager, receiver, sender }
    }
}

impl From<LichessEvent> for Event {
    fn from(value: LichessEvent) -> Self {
        Event::Lichess(value)
    }
}

impl From<TwitchEvent> for Event {
    fn from(value: TwitchEvent) -> Self {
        Event::Twitch(value)
    }
}
