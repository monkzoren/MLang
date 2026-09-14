#!/bin/sh
# Replay-mode checks for chat.ml. No network, no container, no timing —
# ⎆ reads ▷ frames from stdin, so the whole bot is testable as a pure
# function of its input (SPEC §5.5).
#
#     sh chat/test.sh [path-to-mlang]
set -e
MLANG=${1:-./compiler/target/release/mlang}
HERE=$(dirname "$0")
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
fail=0

frame() { printf '\342\226\267 POST %s %s\n%s\n' "$1" "$(printf %s "$2" | wc -c)" "$2"; }

check() { # name expected-substring actual
  if printf %s "$3" | grep -qF "$2"; then
    echo "  ok   $1"
  else
    echo "  FAIL $1"; echo "       wanted: $2"; echo "       got:    $3"; fail=1
  fi
}

run() { # token frames... -> output
  cp "$HERE/chat.ml" "$TMP/c.ml"
  tok=$1; shift
  printf %s "$*" | "$MLANG" run "$TMP/c.ml" "$tok" "$TMP/c.ml"
}

echo "chat.ml:"
"$MLANG" check "$HERE/chat.ml" >/dev/null

out=$(run tok "$(frame /say hello)")
check "answers a known pattern" "Hello. I am a grid" "$out"

out=$(run tok "$(frame /say 'what is xyzzy')")
check "falls back when it does not know" "I do not know that one yet" "$out"

out=$(run tok "$(frame /teach 'tok|xyzzy|A magic word.')$(frame /say 'say xyzzy')")
check "learns, and answers on the new rule" "A magic word." "$out"

out=$(run tok "$(frame /teach 'wrong|x|y')")
check "refuses a bad token" "refused: bad token" "$out"

# An unset TEACH_TOKEN reaches the grid as an empty argument, which must not
# be treated as a token anyone can match.
out=$(run "" "$(frame /teach '|x|Anyone could have written this.')")
check "refuses to learn with no token configured" "not configured" "$out"

out=$(run tok "$(frame /teach 'tok||empty pattern')")
check "refuses an empty pattern" "refused:" "$out"

out=$(run tok "$(frame /teach 'tok|nope')")
check "refuses a malformed body" "expected token" "$out"

# The loom weaves the whole text before accepting it, so a rule that would
# break the program changes nothing and the grid keeps answering.
out=$(run tok "$(frame /teach 'tok|brace|a ] bracket')$(frame /say hello)")
check "survives a rule that would not weave" "Hello. I am a grid" "$out"

exit $fail
