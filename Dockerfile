FROM rust:1.89.0-alpine
WORKDIR /app
RUN apk add openssl-dev openssl-libs-static pkgconfig musl-dev

COPY . .
RUN cargo build --release
ENTRYPOINT ["patchdog"]