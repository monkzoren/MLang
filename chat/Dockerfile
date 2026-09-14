# The grid chatbot. Build context is the repository root:
#     docker build -f chat/Dockerfile -t grid-chat .
#
# Two stages, because the toolchain is only needed to produce one 4 MB
# self-contained binary — the standard library is welded into it, so the
# runtime image carries no MLang source but the bot's own program.
FROM rust:1-slim AS build
WORKDIR /src
# vm.rs welds std/*.ml into the binary with include_str!, so both are needed.
COPY compiler/ compiler/
COPY std/ std/
# No default features: `gui` pulls in minifb/X11 for the canvas ops, which a
# server never opens.
RUN cargo build --release --no-default-features --manifest-path compiler/Cargo.toml

FROM python:3.12-slim
COPY --from=build /src/compiler/target/release/mlang /usr/local/bin/mlang
WORKDIR /app
COPY chat/serve.py chat/chat.ml ./
# LIVE_SOURCE is the program the bot has become; mount a volume there or a
# restart forgets everything it was taught.
ENV PORT=8080 \
    GRID_PORT=4321 \
    MLANG_BIN=/usr/local/bin/mlang \
    SEED_SOURCE=/app/chat.ml \
    LIVE_SOURCE=/data/chat.ml \
    PYTHONUNBUFFERED=1
RUN useradd -m grid && mkdir -p /data && chown grid:grid /data
USER grid
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
  CMD python3 -c "import urllib.request,os;urllib.request.urlopen('http://127.0.0.1:'+os.environ['PORT']+'/').read()"
CMD ["python3", "serve.py"]
