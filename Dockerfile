# See https://hub.docker.com/_/rust/

FROM rust:latest as builder
WORKDIR /usr/src/cizrna
COPY . .
RUN cargo install --path .

FROM registry.access.redhat.com/ubi9-minimal:latest
RUN microdnf install -y compat-openssl\*
COPY --from=builder /usr/local/cargo/bin/cizrna /usr/local/bin/cizrna
# When running this container interactively, use `-v .:/mnt/cizrna:Z`
# to mount the current directory in the host to the container working dir.
VOLUME ["/mnt/cizrna"]
WORKDIR "/mnt/cizrna"
CMD ["cizrna"]
