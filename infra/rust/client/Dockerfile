FROM rust:1.62

RUN mkdir /pir
WORKDIR /pir

COPY . .

VOLUME /pir/data

RUN cargo install --path ./infra/rust/client

ENV USERNAME poc-test@mail.com
ENV PWD poc-pwd
ENV CONFIG /pir/data/local-configs.yml

CMD ["sh", "-c", "pir-client --config ${CONFIG} --username ${USERNAME} --password ${PWD}"]
