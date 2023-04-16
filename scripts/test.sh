#!/usr/bin/env bash

source "./scripts/common.sh"

set -e

export LICHESS_AUTH="lip_DL8luOSdHTY5TilfCY7G"
export LICHESS_ID="twitch-bot-blue"
export TWITCH_ACCOUNT="brongle"

export RUST_LOG=info

function main() {
    echo "Running ChatPlaysChess test stream!"

    setup
    make_config

    run_window &
    window_pid=$!

    run_app
    cleanup
}

function run_window() {
    cargo run --release --example window -- $config_file
}

function cleanup() {
    echo "Cleaning up on exit."
    
    if [ -d "$runtime_dir" ]; then
        echo "Removing runtime directory."
        rm -rf $runtime_dir
    fi

    if [ ! -z "$window_pid" ] && kill -0 "$window_pid" 2>/dev/null; then
        echo "Stopping window."
        kill -9 $window_pid
    fi
}

trap cleanup EXIT

main
