# chat-plays-chess
An automated and user driven chess streaming application.

## How to run

Both macOS and Linux are supported.

Install Rust and cargo:
`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

Install ffmpeg and jq:
- macOS: `brew install ffmpeg jq`
- linux (Ubuntu, Debian, etc): `sudo apt-get install ffmpeg jq`

Export the following environment variables.
```bash
# See: https://lichess.org/account/oauth/token/create
# Needs to have challenge:read, challenge:write and bot:play.
export LICHESS_AUTH="<lichess access token>" # Required
export LICHESS_ID="<lichess bot user id>" # Required
# Must be lowercased.
export TWITCH_ACCOUNT="<twitch account name>" # Required
export TWITCH_STREAM_KEY="<twitch stream key>" # Only required if livestreaming.
# See: https://stream.twitch.tv/ingests/
export TWITCH_INGESTION_SERVER="<closest twitch ingestion server>" # Only required if livestreaming.
```

Finally run `./script/run.sh stream` if live streaming or `./scripts/run.sh test` to stream to a local window.
