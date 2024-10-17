FROM  ubuntu:22.04
LABEL MANTAINER="GeniusLive"

ARG CI_JOB_TOKEN=local

# Setting bash as the default shell in this container
SHELL ["/bin/bash", "-c"] 

# Environment variable to configure timezone
ENV TZ=Etc/UTC \
    FFMPEG_VERSION=5.1 \
    GSTREAMER_VERSION=1.24.2

WORKDIR /tmp/workdir

RUN apt update \
    && apt install -y tzdata \
    && rm -rf /var/lib/apt/lists/*

RUN apt update \
    && apt install -y \
    autoconf \
    build-essential \
    cmake \
    curl \
    dvb-apps \
    fonts-open-sans \
    g++ \
    git \
    liba52-0.7.4-dev \
    libaom-dev \
    libasound2-dev \
    libass-dev \
    libavc1394-dev \
    libavcodec-dev \
    libavdevice-dev \
    libavformat-dev \
    libavutil-dev \
    libcairo2-dev \
    libcdio-cdda-dev \
    libcdio-dev \
    libcdio-paranoia-dev \
    libdrm-dev \
    libfaad-dev \
    libfdk-aac-dev \
    libfreetype6-dev \
    libgl1-mesa-dev \
    libiec61883-dev \
    libjack-dev \
    libjpeg62-dev \
    libmad0-dev \
    libmp3lame-dev \
    libnghttp2-dev \
    libogg-dev \
    libopenal-dev \
    libopencore-amrnb-dev \
    libopencore-amrwb-dev \
    libopus-dev \
    libpango1.0-dev \
    libpng-dev \
    libpulse-dev \
    libraw1394-dev \
    librtmp-dev \
    libsdl2-dev \
    libsoup2.4-dev \
    libsrt-openssl-dev \
    libssl-dev \
    libswscale-dev \
    libtheora-dev \
    libtool \
    libvorbis-dev \
    libvpx-dev \
    libwebp-dev \
    libx264-dev \
    libx265-dev \
    libxcb-shape0-dev \
    libxcb-shm0-dev \
    libxcb-xfixes0-dev \
    libxv-dev \
    libxvidcore-dev \
    mesa-utils \
    pkg-config \
    python-gi-dev \
    scons \
    wget \
    x11proto-gl-dev \
    x11proto-video-dev \
    yasm \
    zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

## Build ffmpeg https://ffmpeg.org/
RUN  \
    DIR=/tmp/ffmpeg && mkdir -p ${DIR} && cd ${DIR} && \
    curl -sLO https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.bz2 && \
    tar -jx --strip-components=1 -f ffmpeg-${FFMPEG_VERSION}.tar.bz2 && \
    ./configure \
    --disable-debug \
    --disable-doc \
    --disable-ffplay \
    --enable-fontconfig \
    --enable-gpl \
    --enable-libaom \
    --enable-libass \
    --enable-libfdk_aac \
    --enable-libfreetype \
    --enable-libmp3lame \
    --enable-libopencore-amrnb \
    --enable-libopencore-amrwb \
    --enable-libopus \
    --enable-libsrt \
    --enable-libtheora \
    --enable-libvorbis \
    --enable-libvpx \
    --enable-libwebp \
    --enable-libx264 \
    --enable-libx265 \
    --enable-libxcb \
    --enable-libxvid \
    --enable-nonfree \
    --enable-openssl \
    --enable-postproc \
    --enable-shared \
    --enable-small \
    --enable-version3 \
    --extra-libs=-ldl \
    --extra-libs=-lpthread && \
    make -j && \
    make install && \
    ldconfig && \
    cd / && \
    rm -rf ${DIR}

ENV LD_LIBRARY_PATH=/usr/local/lib:/usr/local/lib64

RUN apt update \
    && apt remove -y \
    meson \
    && apt install -y \
    python3-pip \
    flex \
    bison \
    && pip3 install meson ninja tomli \
    && rm -rf /var/lib/apt/lists/*

# Install curl and cargo
RUN curl https://sh.rustup.rs -sSf > rustup.sh \
    && chmod +x rustup.sh \
    && ./rustup.sh -y \
    && rm -f rustup.sh

ENV PATH="/root/.cargo/bin:${PATH}"

RUN cargo install cargo-c cargo-cache \
    && cargo-cache -r all -a

###################################################################################################
# GStreamer clone
RUN git clone --depth 1 --branch ${GSTREAMER_VERSION} https://github.com/GStreamer/gstreamer.git

# GStreamer build
RUN cd /tmp/workdir/gstreamer \
    && meson setup --wipe --wrap-mode=nodownload \
    -Dlibav=enabled \
    -Dgood=enabled \
    -Dbad=enabled \
    -Dugly=enabled \
    -Dgpl=enabled \
    -Dgst-plugins-bad:rtmp=enabled \
    -Dgst-plugins-bad:srt=enabled \
    -Dgst-examples=disabled \
    --buildtype release \
    builddir \
    && meson compile -C builddir \
    && cd builddir \
    && meson install \
    && ldconfig \
    && cd /tmp/workdir \
    && rm -rf gstreamer

# GStreamer Rust plugin
RUN rustup default 1.79.0

RUN git clone --depth 1 --branch gstreamer-${GSTREAMER_VERSION} https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs.git  \
    && cd gst-plugins-rs \
    && cargo build --release \
    && cd target/release \
    && mv *.so /usr/local/lib/x86_64-linux-gnu/gstreamer-1.0/ \
    && cd /tmp/workdir \
    && rm -rf gst-plugins-rs \
    && cargo-cache -r all -a

RUN apt update \
    && apt install -y \
    clang \
    curl \
    git-lfs \
    protobuf-compiler \
    && rustup component add rust-analyzer \
    && cargo install cargo-nextest --force --locked

