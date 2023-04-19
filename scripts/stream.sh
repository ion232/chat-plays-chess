source "./scripts/common.sh"

function run_twitch_stream() {
    local stream_key="$TWITCH_STREAM_KEY"
    local ingestion_server="$TWITCH_INGESTION_SERVER"

    live_stream "rtmp://$ingestion_server/app/$stream_key"
}

function live_stream() {
    local audio_input="$(default_audio_input)"
    local audio_options="-c:a aac"

    local video_input="-f image2pipe -i $video_fifo"
    local video_options="-c:v libx264 -preset ultrafast -pix_fmt yuv420p -f flv -r 30"

    local rtmp_endpoint="$1"

    # echo "Command: ffmpeg $audio_input $video_input $audio_options $video_options $rtmp_endpoint"
    ffmpeg -analyzeduration 100M -probesize 500M $audio_input $video_input $audio_options $video_options $rtmp_endpoint
}

function default_audio_input() {
    if [ $kernel_name = "Linux" ]; then
        echo "-f alsa -i hw:0"
    elif [ $kernel_name = "Darwin" ] ; then
        echo "-f avfoundation -i :default"
    fi
}
