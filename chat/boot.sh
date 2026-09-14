#!/bin/sh
# The only part of this application that is not MLang, and it exists for one
# reason: /data outlives the image.
#
# The program IS the memory here — a lesson is a line of source — so a volume
# that persists what the bot learned necessarily persists the code it learned
# in. Seeding only when the file is absent, which is the obvious thing to
# write, pins the bot to whatever version first landed on the volume: every
# later image is built, deployed, and silently ignored.
#
# So the volume keeps two files. chat.ml is what the bot has become. seed.ml
# is the image's program as it stood when that began. When a new image brings
# a different seed, those three are exactly a three-way merge — and the loom
# already knows how to do that, because it is the same question a patch asks:
# the code the image brings, the lines the grid learned, and an honest account
# of anything that genuinely collides.
#
# A merge that fails leaves the volume untouched and does not advance seed.ml,
# so the next boot tries again rather than compounding. A bot that keeps
# answering with old code is a far better outcome than one that does not boot.
set -e

SEED=/app/chat.ml
LIVE=/data/chat.ml
BASE=/data/seed.ml
PORT=${PORT:-8080}

rules() { grep -c '^ ⟨«' "$1" 2>/dev/null || echo 0; }

if [ ! -f "$LIVE" ]; then
  cp "$SEED" "$LIVE"; cp "$SEED" "$BASE"
  echo "⇓ seeded /data from the image ($(rules "$LIVE") rules)" >&2

elif [ ! -f "$BASE" ]; then
  # A volume written before seed.ml was recorded. There is no base, so there
  # is no sound merge; guessing one would silently mangle the program. Keep a
  # copy of what was there and say so, rather than pretending.
  STAMP=$(date +%Y%m%d-%H%M%S)
  cp "$LIVE" "/data/chat.ml.$STAMP.bak"
  cp "$SEED" "$LIVE"; cp "$SEED" "$BASE"
  echo "⚠ /data had no recorded seed, so its lessons could not be carried" >&2
  echo "  forward. The old program is kept at /data/chat.ml.$STAMP.bak;" >&2
  echo "  re-teach anything it held. This happens once." >&2

elif ! cmp -s "$SEED" "$BASE"; then
  echo "⇓ the image brings a new program — merging what this grid became" >&2
  if /usr/local/bin/mlang merge "$BASE" "$LIVE" "$SEED" > /data/.merged 2>/data/.why; then
    WAS=$(rules "$LIVE")
    mv /data/.merged "$LIVE"; cp "$SEED" "$BASE"
    echo "⟡ merged: $WAS rules in, $(rules "$LIVE") out" >&2
  else
    # seed.ml is deliberately not advanced: the next boot retries.
    rm -f /data/.merged
    echo "✗ the merge did not hold, so /data keeps the program it had:" >&2
    sed 's/^/  /' /data/.why >&2
  fi
  rm -f /data/.why
fi

# Positional arguments, built with set -- because empty ones must survive:
# blank means "keep chat.ml's default", and a collapsed argument would shift
# every later one into the wrong slot.
set -- serve --parallel "$LIVE" "$PORT" "$TEACH_TOKEN" "$LIVE"
KEY=${MODEL_API_KEY:-${DEEPSEEK_API_KEY:-$ANTHROPIC_API_KEY}}
if [ -n "$KEY" ]; then
  set -- "$@" "$KEY" "$MODEL_NAME" "$MODEL_URL" "$MODEL_DIALECT" "$MODEL_VERSION"
else
  echo "※ no model key set — the grid answers what it knows and learns nothing" >&2
fi
exec /usr/local/bin/mlang "$@"
