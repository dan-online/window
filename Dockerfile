# Run application
FROM ubuntu:noble

WORKDIR /app

RUN apt update -y && apt install -y libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev libpostproc-dev libswresample-dev libswscale-dev

COPY ./target/release/window /usr/local/bin/window

CMD ["/usr/local/bin/window"]