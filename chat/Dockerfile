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
RUN useradd -m grid && mkdir -p /data && chown grid:grid /data
COPY --from=build /src/compiler/target/release/mlang /usr/local/bin/mlang
COPY chat/chat.ml /app/chat.ml
USER grid
# Reachable from off the machine — which is why the loom's HTTP routes stay
# shut here (SPEC §5.5). The bot still re-weaves itself; nobody else can.
ENV MLANG_HOST=0.0.0.0
EXPOSE 8080
# ⍆ honours HTTP_PROXY, and a proxy set for outbound traffic would answer
# this instead of the grid.
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
  CMD env -u HTTP_PROXY -u http_proxy mlang eval '«http://127.0.0.1:8080/»⍆⌫'
# The one line here that is not MLang: seed the volume on first boot, because
# /data is mounted over whatever the image put there.
CMD ["sh", "-c", "[ -f /data/chat.ml ] || cp /app/chat.ml /data/chat.ml; exec mlang serve /data/chat.ml 8080 \"$TEACH_TOKEN\" /data/chat.ml"]
