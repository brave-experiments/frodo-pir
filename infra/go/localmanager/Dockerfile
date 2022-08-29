FROM golang:1.16-alpine

WORKDIR /app

COPY infra/go/localmanager .

VOLUME /pir/data

RUN go build -o localmanager

CMD [ "./localmanager" ]
