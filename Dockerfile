# Run application
FROM ubuntu:noble

WORKDIR /build

RUN apt update -y && apt install -y python3 python3-pip libssl-dev libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev libpostproc-dev libswresample-dev libswscale-dev --no-install-recommends && rm -rf /var/lib/apt/lists/*

ADD https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp /usr/local/bin/yt-dlp

RUN chmod a+rx /usr/local/bin/yt-dlp

# ssl fix for yt-dlp
# RUN mkdir -p /etc/ssl/certs && ln -s /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-bundle.crt
RUN python3 -m pip install --no-cache-dir --upgrade certifi --break-system-packages

COPY target/release/window /usr/local/bin/window

ENTRYPOINT ["/usr/local/bin/window"]