FROM rust:1.62

RUN mkdir /pir
WORKDIR /pir

COPY ./rust /pir
RUN mkdir test_data
VOLUME /pir/test_data

RUN cargo install --path ./infra/server-instance

ENV ENV local
ENV PORT 8080
ENV SHARDS_INTERVAL 0-1
ENV SHARD_DIR /pir/test_data/local_buckets
ENV OPRF_KEY VBiS3Zlp4UjLXLf9nw4GtU0j5LVfA9T+0u31skECPAY=

CMD ["sh", "-c", "ENV=local PORT=${PORT} SHARDS_INTERVAL=${SHARDS_INTERVAL} SHARD_DIR=${SHARD_DIR} OPRF_KEY=${OPRF_KEY} server-instance"]
