FROM rust:1.62

RUN mkdir /pir
WORKDIR /pir

COPY . .

VOLUME /pir/data

RUN cargo install --path ./infra/rust/server

ENV ENV local
ENV PORT 8080
ENV SHARDS_INTERVAL 0-1
ENV SHARD_DIR /pir/data/local-env/local_buckets
ENV OPRF_KEY VBiS3Zlp4UjLXLf9nw4GtU0j5LVfA9T+0u31skECPAY=

CMD ["sh", "-c", "ENV=local PORT=${PORT} SHARDS_INTERVAL=${SHARDS_INTERVAL} SHARD_DIR=${SHARD_DIR} OPRF_KEY=${OPRF_KEY} pir-server"]
