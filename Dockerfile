FROM rust:1.62-alpine3.16 as builder

RUN apk add --no-cache musl-dev \
    mysql-dev postgresql-dev sqlite-dev

RUN adduser -D moonramp --uid 1337
ENV SQLITE3_STATIC=1

WORKDIR /opt/moonramp
COPY . .
RUN cargo build --release

FROM scratch

ENV LOGNAME=moonramp
COPY --from=builder /etc/passwd /etc/passwd
USER moonramp

WORKDIR /home/moonramp
COPY --from=builder /opt/moonramp/target/release/moonramp-migration /usr/bin/
ENTRYPOINT ["moonramp-migration"]
