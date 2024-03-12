FROM rust:1.76 as builder

WORKDIR /app
COPY src /app/src
COPY templates /app/templates
COPY Cargo.toml /app/Cargo.toml
COPY Cargo.lock /app/Cargo.lock

RUN apt update
RUN apt-get install -y libclang-dev cmake

RUN cargo build --release

# Runtime
FROM rust:1.76 as runtime

WORKDIR /app
COPY --from=builder /app/target/release/weather-app /app/weather-app
COPY templates /app/templates
COPY static /app/static
COPY weather-codes.json .

EXPOSE 3000

CMD ["ls", "-l"]
