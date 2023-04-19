#!/usr/bin/env bash

source "./scripts/common.sh"
source "./scripts/stream.sh"

set -e

export RUST_LOG=debug

function main() {
    echo "Running ChatPlaysChess test stream! $1"

    setup
    make_config
    build_app

    if [ "$1" == "test" ]; then
        echo "Running window."
        run_window &
        window_pid=$!
    elif [ "$1" == "stream" ]; then
        echo "Running live stream."
        run_twitch_stream &
        twitch_stream_pid=$!
    else
        echo "Usage: ./scripts/run.sh [test|stream]"
        return
    fi

    run_app
}

function run_window() {
    cargo run --release --example window -- $config_file
}

function cleanup() {
    echo "Cleaning up on exit."
    
    if [ ! -z "$window_pid" ] && kill -0 "$window_pid" 2>/dev/null; then
        echo "Stopping window."
        kill -9 $window_pid
    fi

    if [ ! -z "$twitch_stream_pid" ] && kill -0 "$twitch_stream_pid" 2>/dev/null; then
        echo "Stopping twitch stream."
        kill -9 $twitch_stream_pid
    fi

    if [ -d "$runtime_dir" ]; then
        echo "Removing runtime directory."
        rm -rf $runtime_dir
    fi
}

trap cleanup EXIT

main $1
