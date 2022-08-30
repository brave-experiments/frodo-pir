FROM rust:1.62

RUN mkdir /frodo-pir
WORKDIR /frodo-pir

COPY . .

CMD cargo clean && \ 
    cargo build --release && \
    cargo test --release -- --nocapture