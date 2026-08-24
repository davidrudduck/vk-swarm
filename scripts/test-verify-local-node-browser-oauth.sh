#!/usr/bin/env bash
# Executable test for scripts/verify-local-node-browser-oauth.sh.
set -euo pipefail

FAKE_PID=''
FAKE_URL=''
FAKE_READY=''

cleanup_fake() {
  if [ -n "${FAKE_PID:-}" ]; then
    kill "$FAKE_PID" 2>/dev/null || true
    wait "$FAKE_PID" 2>/dev/null || true
    FAKE_PID=''
  fi
  if [ -n "${FAKE_READY:-}" ]; then
    rm -f "$FAKE_READY"
    FAKE_READY=''
  fi
}
trap cleanup_fake EXIT

start_fake() {   # $1 = fixture; sets FAKE_URL and FAKE_PID in THIS shell
  local ready=0 port
  cleanup_fake
  FAKE_READY=$(mktemp "${TMPDIR:-/tmp}/vlnbo-ready.XXXXXX")
  FIXTURE="$1" READY_FILE="$FAKE_READY" node -e '
    const fs = require("fs");
    const http = require("http");
    const f = process.env.FIXTURE;
    const server = http.createServer((req, res) => {
      const url = req.url.split("?")[0];
      const json = (code, body) => { res.writeHead(code, {"content-type":"application/json"});
                                     res.end(body); };
      if (url === "/api/health") return json(200, JSON.stringify({status:"ok"}));
      if (url === "/api/auth/state") {
        const data = {authorized:false, oauth_available:true};
        if (f === "leaky_state") data.access_token = "leaked";
        if (f === "extra_state") data.credentials_path = "/tmp/private";
        return json(200, JSON.stringify({success:true, data,
              error_data:null, message:null}));
      }
      if (url === "/api/info" || url === "/api/projects" || url === "/api/auth/status")
        return json(f === "open_info" ? 200 : 401, "{}");
      if (url === "/api/events") return json(401, "{}");
      if (/^\/api\/logs\/.*\/live$/.test(url)) return json(401, "{}");
      if (f === "spa_fallback") { res.writeHead(200, {"content-type":"text/html"});
                                   return res.end("<!DOCTYPE html><html></html>"); }
      return json(404, JSON.stringify({success:false}));
    });
    server.listen(0, "127.0.0.1", () =>
      fs.writeFileSync(process.env.READY_FILE, String(server.address().port)));
  ' &
  FAKE_PID=$!

  for _ in $(seq 1 50); do
    if [ -s "$FAKE_READY" ]; then ready=1; break; fi
    if ! kill -0 "$FAKE_PID" 2>/dev/null; then break; fi
    sleep 0.1
  done
  if [ "$ready" -ne 1 ]; then
    echo "fake server did not become ready" >&2
    return 1
  fi
  port=$(<"$FAKE_READY")
  FAKE_URL="http://127.0.0.1:$port"
}

run_case() {   # $1 = fixture, $2 = expected exit status, $3 = label
  local status
  start_fake "$1"
  set +e
  bash scripts/verify-local-node-browser-oauth.sh "$FAKE_URL" \
    >"${TMPDIR:-/tmp}/vlnbo.$$.out" 2>&1
  status=$?
  set -e
  cleanup_fake
  if [ "$status" -ne "$2" ]; then
    echo "FAIL $3: expected exit $2, got $status"
    cat "${TMPDIR:-/tmp}/vlnbo.$$.out"
    exit 1
  fi
  echo "PASS $3"
}

run_case compliant    0 'compliant node passes'
run_case open_info    1 'NEGATIVE: /api/info returning 200 must FAIL the verifier'
run_case spa_fallback 1 'NEGATIVE: unknown /api falling through to SPA html must FAIL'
run_case leaky_state  1 'NEGATIVE: auth state leaking a token field must FAIL'
run_case extra_state  1 'NEGATIVE: auth state carrying any extra field must FAIL'
rm -f "${TMPDIR:-/tmp}/vlnbo.$$.out"
echo 'ALL VERIFIER TESTS PASSED'
