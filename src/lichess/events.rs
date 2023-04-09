use async_std::stream::StreamExt;
use lichess_api::model::bot;
use tokio::task::JoinHandle;

use crate::error::Result;

pub enum Event {
    AccountEvent { event: bot::stream::events::Event },
    GameEvent { game_id: String, event: bot::stream::game::Event },
}

impl crate::engine::events::EventSubscriber {
    pub async fn stream_lichess_account_events(&self) -> Result<JoinHandle<()>> {
        let request = bot::stream::events::GetRequest::new();
        let mut stream = self.lichess_context.api.bot_stream_incoming_events(request).await?;

        let sender = self.sender.clone();

        Ok(tokio::task::spawn(async move {
            while let Some(result) = stream.next().await {
                let result = result
                    .map(|event| crate::engine::events::Event::LichessEvent(Event::AccountEvent { event }))
                    .map_err(|e| crate::error::Error::LichessError(e));
                sender.send(result).unwrap_or_default();
            }
        }))
    }

    pub async fn stream_lichess_game_events(&self, game_id: &str) -> Result<JoinHandle<()>> {
        let request = bot::stream::game::GetRequest::new(game_id);
        let mut stream = self.lichess_context.api.bot_stream_board_state(request).await?;

        let sender = self.sender.clone();
        let game_id = game_id.to_string();

        Ok(tokio::task::spawn(async move {
            while let Some(result) = stream.next().await {
                let result = result
                    .map(|event| {
                        crate::engine::events::Event::LichessEvent(Event::GameEvent { game_id: game_id.clone(), event })
                    })
                    .map_err(|e| crate::error::Error::LichessError(e));
                sender.send(result).unwrap_or_default();
            }
        }))
    }
}
