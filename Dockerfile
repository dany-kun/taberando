# From https://kerkour.com/deploy-rust-on-heroku-with-docker
# Do not use the Alpine image with a musl target as the openssl crate is painful to set for this architecture
####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

ARG FIREBASE_URL
ARG FIREBASE_JAR_OVERRIDES

RUN update-ca-certificates

# Create appuser
ENV USER=taberando
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


COPY ./server ./taberando

WORKDIR /taberando

RUN FIREBASE_JAR_OVERRIDES=${FIREBASE_JAR_OVERRIDES} FIREBASE_URL=${FIREBASE_URL} cargo build --release

####################################################################################################
## Final image
####################################################################################################
FROM bitnami/minideb:latest

RUN apt-get update && apt-get install -y openssl libssl-dev
RUN apt-get install ca-certificates
RUN update-ca-certificates


# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /taberando

# Copy our build
COPY --from=builder /taberando/target/release/server ./
# Copy resources
COPY --from=builder /taberando/resources ./resources

# Use an unprivileged user.
USER taberando:taberando

CMD ["/taberando/server"]