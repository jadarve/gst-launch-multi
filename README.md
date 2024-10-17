# gst-launch-multi
A gstreamer pipeline launcher supporting multiple pipelines

## Gst Rust plugin

```bash
cd local
git clone --depth 1 --branch gstreamer-1.24.2 https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs.git
cd gst-plugins-rs
cargo build --release
# release
cp target/release/*.so /usr/local/lib/x86_64-linux-gnu/gstreamer-1.0/
# debug
cp target/debug/*.so /usr/local/lib/x86_64-linux-gnu/gstreamer-1.0/

cargo build; cp target/debug/*.so /usr/local/lib/x86_64-linux-gnu/gstreamer-1.0/
```

## TODOs

* Refactor to use `Arc<Pipeline>`.
* Handle exit correctly with intersrc elements.
* Add tests for CLI arguments.

## Examples

### Basic

```bash
cargo run --bin gst-launch-multi -- \
  --pipeline --name testsrc videotestsrc ! video/x-raw ! tee name=video_tee ! queue ! videoconvert ! autovideosink \
  --pipeline --name testsrc_1 videotestsrc ! video/x-raw ! tee name=video_tee ! queue ! videoconvert ! autovideosink
```

```bash
cargo run --bin gst-launch-multi -- \
  --pipeline --name testsrc videotestsrc ! video/x-raw ! tee name=video_tee ! queue ! videoconvert ! autovideosink video_tee. ! queue ! intersink name=testsrc_raw_video \
  --pipeline --name display_video intersrc name=testsrc_raw_video ! queue ! videoconvert ! textoverlay text="display_video" ! autovideosink
```

```bash
# With netsim failures
gst-launch-1.0 \
  videotestsrc is-live=true pattern=ball motion=wavy animation-mode=frames foreground-color=0xFF0000 ! video/x-raw,width=1280,height=720,framerate=30000/1001 \
  ! queue ! videoconvert ! x264enc bitrate=1000 tune=zerolatency ! video/x-h264 ! h264parse \
  ! queue ! mpegtsmux alignment=7 name=mux \
  ! queue ! netsim name=netsim delay-distribution=uniform delay-probability=0.01 min-delay=10 max-delay=100 drop-probability=0.001 \
  ! queue ! srtsink uri=srt://:7000?mode=listener wait-for-connection=false poll-timeout=-1

gst-launch-1.0 \
  videotestsrc is-live=true pattern=ball motion=wavy animation-mode=frames foreground-color=0xFF0000 ! video/x-raw,width=1280,height=720,framerate=30000/1001 \
  ! queue ! videoconvert ! x264enc bitrate=1000 tune=zerolatency ! video/x-h264 ! h264parse \
  ! queue ! mpegtsmux alignment=7 name=mux \
  ! queue ! srtsink uri=srt://:7000?mode=listener wait-for-connection=false poll-timeout=-1

export GST_DEBUG="intersink:4,utilsrs-stream-producer:4"
cargo run --bin gst-launch-multi -- \
  --pipeline --name ingress uridecodebin uri=srt://127.0.0.1:7000?mode=caller name=decoder \
    decoder. ! video/x-raw ! queue ! videoconvert ! clockoverlay valignment=5 ypos=0.1 ! textoverlay text=Ingress valignment=5 ypos=0.1 ! tee name=video_tee ! queue ! autovideosink video_tee. ! queue ! intersink producer-name=ingress_raw_video \
  \
  --pipeline --name video_link_0 intersrc name=ingress_raw_video producer-name=ingress_raw_video ! queue name=ingress_raw_video_queue max-size-bytes=104857600 max-size-time=20000000000 min-threshold-time=1000000000 ! videoscale ! videoconvert ! clockoverlay valignment=5 ypos=0.25 ! textoverlay text="video_link_0" valignment=5 ypos=0.25 ! tee name=video_tee ! queue ! videoconvert ! autovideosink video_tee. ! queue ! intersink producer-name=video_link_0 \
  \
  --pipeline --name video_link_1 intersrc name=video_link_0 producer-name=video_link_0 ! queue name=ingress_raw_video_queue max-size-bytes=104857600 max-size-time=20000000000 min-threshold-time=1000000000 ! videoscale ! videoconvert ! clockoverlay valignment=5 ypos=0.4 ! textoverlay text="video_link_1" valignment=5 ypos=0.4 ! tee name=video_tee ! queue ! videoconvert ! autovideosink video_tee. ! queue ! intersink producer-name=video_link_1 \
  \
  --pipeline --name video_link_2 intersrc name=video_link_1 producer-name=video_link_1 ! queue name=ingress_raw_video_queue max-size-bytes=104857600 max-size-time=20000000000 min-threshold-time=1000000000 ! videoscale ! videoconvert ! clockoverlay valignment=5 ypos=0.55 ! textoverlay text="video_link_2" valignment=5 ypos=0.55 ! tee name=video_tee ! queue ! videoconvert ! autovideosink video_tee. ! queue ! intersink producer-name=video_link_2

set-property --pipeline video_link_0 --element ingress_raw_video_queue --property min-threshold-time --value 3000000000

get-latency --pipeline ingress
get-latency --pipeline video_link_0
get-latency --pipeline video_link_1
get-latency --pipeline video_link_2
push-latency-event --pipeline video_link_0


# the change in latency in video_link_0 is not reflected in video_link_1 automatically. It needs to be set up manually.
get-latency --pipeline video_link_1
get-latency --pipeline video_link_1 --element ingress_raw_video_queue
set-latency --pipeline video_link_1 --latency-ms 5919
push-latency-event --pipeline video_link_1
get-latency --pipeline video_link_1
```