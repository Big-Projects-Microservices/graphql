FROM rust:1.86-alpine3.21

RUN apk update && apk add --no-cache \
    musl-dev \
    gcc \
    libc-dev \
    make

WORKDIR /usr/src/graphql
COPY . .

RUN cargo build -r

EXPOSE 8000

CMD ["./target/release/graphql"]
