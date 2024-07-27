# Run application
FROM ubuntu:noble

WORKDIR /build

RUN apt update -y && apt install -y python3 python3-pip libssl-dev libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev libpostproc-dev libswresample-dev libswscale-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*

ADD https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp /usr/local/bin/yt-dlp

RUN chmod a+rx /usr/local/bin/yt-dlp

# ssl fix for yt-dlp, there's probably a better way to do this
RUN python3 -m pip install --no-cache-dir --upgrade certifi --break-system-packages

# for now, we'll just copy the binary, .framerate is only in ubuntu 24 with libavformat
COPY target/release/window /usr/local/bin/window

ENTRYPOINT ["/usr/local/bin/window"]