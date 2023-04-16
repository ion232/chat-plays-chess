#!/usr/bin/env bash

set -e

function check_env_vars() {
    local missing_var=false

    if [ -z "$LICHESS_AUTH" ]; then
        echo "Missing LICHESS_AUTH environment variable."
        missing_var=true
    fi

    if [ -z "$LICHESS_ID" ]; then
        echo "Missing LICHESS_ID environment variable."
        missing_var=true
    fi

    if [ -z "$TWITCH_ACCOUNT" ]; then
        echo "Missing TWITCH_ACCOUNT environment variable."
        missing_var=true
    fi

    if [ -z "$INGESTION_SERVER" ]; then
        echo "Missing INGESTION_SERVER environment variable."
        missing_var=true
    fi

    if [ -z "$STREAM_KEY" ]; then
        echo "Missing STREAM_KEY environment variable."
        missing_var=true
    fi

    if [ $missing_var ]; then
        return 1
    fi
}


function app() {
    local project_name=chat-plays-chess
    cargo run --release --package $project_name --bin $project_name -- $app_config
}

function cleanup() {
    echo "Cleaning up on exit."
    
    if [ -d "$runtime_dir" ]; then
        echo "Deleting runtime directory."
        rm -rf $runtime_dir
    fi

    if [ ! -z "$app_pid" ]; then
        echo "Stopping app."
        kill -9 $app_pid
    fi

    if [ ! -z "$livestream_pid" ]; then
        echo "Stopping livestream."
        kill -9 $livestream_pid
    fi
}

function main() {
    echo "Running ChatPlaysChess livestream!"

    check_env_vars
    setup
    make_config

    app &
    app_pid=$!

    livestream &
    livestream_pid=$!

    wait $app_pid
    unset app_pid

    kill -9 $livestream_pid
    unset $livestream_pid
}

trap cleanup EXIT ERR INT TERM

main
