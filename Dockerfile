# See https://hub.docker.com/_/rust/

FROM rust:latest as builder
WORKDIR /usr/src/acorns
COPY . .
RUN cargo install --path .

FROM registry.access.redhat.com/ubi9-minimal:latest
RUN microdnf install -y compat-openssl\*
COPY --from=builder /usr/local/cargo/bin/acorns /usr/local/bin/acorns
# When running this container interactively, use `-v .:/mnt/acorns:Z`
# to mount the current directory in the host to the container working dir.
VOLUME ["/mnt/acorns"]
WORKDIR "/mnt/acorns"
CMD ["acorns"]
