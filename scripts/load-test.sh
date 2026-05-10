#!/usr/bin/env bash
# Drives the local service in a way that stresses memory: many parallel
# requests + a configurable fraction of clients that abort mid-flight
# (simulating real users who close the tab before the response lands).
#
# Env:
#   SERVICE_URL    base URL (default: http://127.0.0.1:8080)
#   PARALLEL       max concurrent in-flight requests (default: 8)
#   PASSES         how many times to loop the token list (default: 1)
#   ABORT_RATE     percent of requests that hard-abort early (default: 30)
#   ABORT_TIMEOUT  seconds before aborting those requests (default: 0.3)

set -uo pipefail

SERVICE_URL="${SERVICE_URL:-http://127.0.0.1:8080}"
PARALLEL="${PARALLEL:-8}"
PASSES="${PASSES:-1}"
ABORT_RATE="${ABORT_RATE:-30}"
ABORT_TIMEOUT="${ABORT_TIMEOUT:-0.3}"

TOKENS=(
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS9oZWFkLmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xX2VkaXQyLndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8yLndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8zLndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS80LndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS81LndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS82LndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS83LndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS84LndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS85LndlYnA=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xMC53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xMS53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xMi53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xMy53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xNC53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xNS53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xNi53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9waG90b3NfMS8xNy53ZWJw
    aHR0cHM6Ly90Lm1lL2kvdXNlcnBpYy8zMjAvNDViZTJDSjkxV285TDI0anktS1ctY1NzZWpBODhIMFhQTjBjWGhRTjdQcy5qcGc_dGltZXN0YW1wPTE3Nzc3MjEzMjE=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS8tMS5qcGVn
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS8wLmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS8xLmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS8yLmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS8zLmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS80LmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS81LmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS82LmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS83LmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZS84LmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvaGVhZGVyLmpwZWc=
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvMS53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvMi53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvMy53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvNC53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvNS53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvNi53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvNy53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvOC53ZWJw
    aHR0cHM6Ly9zdG9yYWdlLnRpa2l0a28uZGV2L2ltYWdlcy9aYWtvcGFuZTIvOS53ZWJw
)

hit() {
    local idx="$1"
    local token="$2"
    local kind="full"
    local timeout="60"
    # Roll the dice — abort some fraction of requests mid-flight to simulate
    # users closing the tab while the worker is still processing.
    if (( RANDOM % 100 < ABORT_RATE )); then
        kind="abort"
        timeout="$ABORT_TIMEOUT"
    fi
    local code
    code=$(curl -s -o /dev/null --max-time "$timeout" -w '%{http_code}' \
        "$SERVICE_URL/mirror/$token" 2>/dev/null) || code="---"
    printf '[%4d] %-5s %-4s %s\n' "$idx" "$kind" "$code" "$token"
}

total=$(( ${#TOKENS[@]} * PASSES ))
echo "hitting ${SERVICE_URL}/mirror/<token>: ${total} requests, parallel=${PARALLEL}, abort_rate=${ABORT_RATE}%"

i=0
for ((pass=1; pass<=PASSES; pass++)); do
    for token in "${TOKENS[@]}"; do
        i=$((i + 1))
        hit "$i" "$token" &
        # Cap concurrency: when a wave of $PARALLEL is in flight, wait it out.
        if (( i % PARALLEL == 0 )); then
            wait
        fi
    done
done
wait

echo "done: $i requests"
