# See https://hub.docker.com/_/rust/

FROM rust:latest as builder
WORKDIR /usr/src/cizrna
COPY . .
RUN cargo install --path .

FROM registry.access.redhat.com/ubi9-minimal:latest
RUN microdnf install -y compat-openssl\*
COPY --from=builder /usr/local/cargo/bin/cizrna /usr/local/bin/cizrna
ENTRYPOINT ["cizrna"]
