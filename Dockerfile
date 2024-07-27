# Run application
FROM ubuntu:noble

WORKDIR /app

COPY ./target/release/window /usr/local/bin/window

CMD ["/usr/local/bin/window"]