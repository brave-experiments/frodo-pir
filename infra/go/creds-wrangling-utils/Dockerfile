FROM golang:1.16-alpine

WORKDIR /app

COPY infra/go/creds-wrangling-utils .

VOLUME /pir/data

RUN go build -o prepare-buckets

CMD [ "./prepare-buckets" ]
