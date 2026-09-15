# The grid chatbot. Build context is the repository root:
#     docker build -f chat/Dockerfile -t grid-chat .
#
# The runtime carries one ~4 MB binary and one .ml file. No interpreter, no
# web server, no sidecar: the program below is the whole application, and it
# re-weaves itself with ⟐ and ⟡ (SPEC §4.7) rather than being patched from
# outside.
FROM rust:1-slim-bookworm AS build
WORKDIR /src
# vm.rs welds std/*.ml into the binary with include_str!, so both are needed.
COPY compiler/ compiler/
COPY std/ std/
# No default features: `gui` pulls in minifb/X11 for the canvas ops, which a
# server never opens.
RUN cargo build --release --no-default-features --manifest-path compiler/Cargo.toml

# Pinned to the same Debian release as the builder: the binary is linked
# against the builder's glibc and has to run on the runtime's.
FROM debian:bookworm-slim
# ca-certificates: ⍄ speaks TLS through rustls with the *platform* trust
# store, and debian-slim ships none. Without this the learner's call to the
# model fails the handshake and the bot silently never learns anything.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd -m grid && mkdir -p /data && chown grid:grid /data
COPY --from=build /src/compiler/target/release/mlang /usr/local/bin/mlang
# The primer the learner hands the model when it asks for a skill: the
# language reference, from the binary that will run what comes back, so
# the two can never disagree about which operations exist.
RUN mkdir -p /app && mlang ops > /app/primer.txt
COPY chat/chat.ml /app/chat.ml
COPY chat/boot.sh /usr/local/bin/boot
USER grid
# Reachable from off the machine — which is why the loom's HTTP routes stay
# shut here (SPEC §5.5). The bot still re-weaves itself; nobody else can.
ENV MLANG_HOST=0.0.0.0
# The port the grid listens on, and the one the healthcheck probes, are the
# same variable — so a deployment that moves one cannot leave the other
# behind. A proxy pointed at a port nothing is on is a 502 from a container
# that reports itself healthy, which is a miserable thing to debug.
ENV PORT=8080
EXPOSE 8080
# ⍆ honours HTTP_PROXY, and a proxy set for outbound traffic would answer
# this instead of the grid.
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
  CMD env -u HTTP_PROXY -u http_proxy mlang eval "«http://127.0.0.1:$PORT/»⍆⌫"
# The one line here that is not MLang: seed the volume on first boot, because
# /data is mounted over whatever the image put there.
# /data outlives the image, and here the program is the memory — so a new
# image cannot simply overwrite what the bot became, and must not be
# ignored by it either. boot merges the two with the loom's own diff3.
CMD ["/usr/local/bin/boot"]
