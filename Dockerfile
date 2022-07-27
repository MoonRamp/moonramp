FROM rust:1.62-alpine3.16 as builder

ARG artifact_mode="--release"
ARG artifact_path="release"

RUN rustup target add wasm32-wasi

RUN apk add --no-cache --repository=http://dl-cdn.alpinelinux.org/alpine/edge/testing/ \
    binaryen \
    musl-dev \
    mysql-dev \
    postgresql-dev \
    sqlite-dev \
    wabt

RUN adduser -D moonramp --uid 1337
ENV SQLITE3_STATIC=1

WORKDIR /opt/moonramp
COPY . .
RUN cargo build $artifact_mode

WORKDIR /opt/moonramp/programs/default-sale
RUN cargo build $artifact_mode --target wasm32-wasi
ENV WASM_BIN=/opt/moonramp/programs/default-sale/target/wasm32-wasi/$artifact_path/moonramp_program_default_sale.wasm
RUN wasm-opt -Os -o $WASM_BIN $WASM_BIN
RUN wasm-validate $WASM_BIN

FROM scratch

ARG artifact_path="release"

ENV LOGNAME=moonramp
COPY --from=builder /etc/passwd /etc/passwd
USER moonramp

WORKDIR /home/moonramp
COPY --from=builder /opt/moonramp/target/$artifact_path/moonramp /usr/bin/
COPY --from=builder /opt/moonramp/target/$artifact_path/moonramp-keygen /usr/bin/
COPY --from=builder /opt/moonramp/target/$artifact_path/moonramp-migration /usr/bin/
COPY --from=builder /opt/moonramp/target/$artifact_path/moonramp-prgm-harness /usr/bin/
COPY --from=builder /opt/moonramp/target/$artifact_path/moonrampctl /usr/bin/

COPY --from=builder /opt/moonramp/programs/default-sale/target/wasm32-wasi/$artifact_path/moonramp_program_default_sale.wasm /home/moonramp
ENTRYPOINT ["moonramp"]
