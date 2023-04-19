kernel_name=$(uname -s)

runtime_dir="./runtime"
config_file="$runtime_dir/config.json"
video_fifo="$runtime_dir/video"

function setup() {
    echo "Setting up runtime."

    # Make runtime directory.
    if [ -d "$runtime_dir" ]; then
        echo "Removing existing runtime."
        rm -rf "$runtime_dir"
    fi

    echo "Creating runtime directory $runtime_dir"
    mkdir "$runtime_dir"

    echo "Creating video fifo at $video_fifo"
    mkfifo "$video_fifo"
}

function make_config() {
    if [ -z "$LICHESS_AUTH" ]; then
        echo "Missing LICHESS_AUTH environment variable."
        return 1
    fi

    if [ -z "$LICHESS_ID" ]; then
        echo "Missing LICHESS_ID environment variable."
        return 1
    fi

    if [ -z "$TWITCH_ACCOUNT" ]; then
        echo "Missing TWITCH_ACCOUNT environment variable."
        return 1
    fi

    local json_base='{
        "lichess": {
            "account": "",
            "access_token": ""
        },
        "twitch": {
            "channel": ""
        },
        "livestream": {
            "video": {
                "fifo": ""
            }
        }
    }'

    local config=$(echo "$json_base" | jq \
        --arg lichess_id "$LICHESS_ID" \
        --arg lichess_auth "$LICHESS_AUTH" \
        --arg twitch_account "$TWITCH_ACCOUNT" \
        --arg video_fifo "$video_fifo" \
        '
        .lichess.account = $lichess_id |
        .lichess.access_token = $lichess_auth |
        .twitch.channel = $twitch_account |
        .livestream.video.fifo = $video_fifo
        ')

    echo "$config" > $config_file
}

function build_app() {
    cargo build --release
}

function run_app() {
    local project_name=chat-plays-chess
    cargo run --release --package $project_name --bin $project_name -- $config_file
}
