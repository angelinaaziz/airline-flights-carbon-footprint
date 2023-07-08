FROM rust:1.69.0
WORKDIR /app
COPY . /app
RUN cargo build --release
CMD ["./target/release/carbon-footprint-cli"]