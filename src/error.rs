#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lichess error: {0}")]
    LichessError(#[from] lichess_api::error::Error),

    #[error("receive error: {0}")]
    ReceiveError(#[from] crossbeam_channel::RecvError),

    #[error("send error: {0}")]
    SendError(#[from] crossbeam_channel::SendError<crate::engine::events::Event>),

    #[error("unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;
