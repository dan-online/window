# Run application
FROM ubuntu:noble

WORKDIR /build

RUN apt update -y && apt install -y libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev libpostproc-dev libswresample-dev libswscale-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*

COPY target/release/window /usr/local/bin/window

ADD https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp /usr/local/bin/yt-dlp

RUN chmod a+rx /usr/local/bin/yt-dlp

ENTRYPOINT ["/usr/local/bin/window"]