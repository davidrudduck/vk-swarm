#!/usr/bin/env bash
# Live two-leg WAL-unlink durability harness.
set -euo pipefail

LEGS="${LEGS-AB}"
MODE="${MODE-full}"
BINARY="${BINARY:-target/release/vks-node-server}"
SCRATCH_ROOT="${SCRATCH_ROOT:-}"
BACKEND_PORT=9012
NODE_PIDS=()
PASS_COUNT=0
FAIL_COUNT=0
declare -A LEG_RESULTS
declare -A LEG_PASS_COUNTS
declare -A LEG_FAIL_COUNTS
declare -A LEG_TOTAL_COUNTS
declare -A LEG_COMPLETION_SENTINELS
RUN_NODE_PID=
BOOT_PID=
TRIP_FIRED=0
WRITE_SESSION_SUCCEEDED=0
STOP_REASON=

unset VK_HIVE_URL VK_NODE_API_KEY VK_NODE_NAME VK_NODE_PUBLIC_URL VK_WAL_GUARD 2>/dev/null || true

log_info() { echo "[$(date +%H:%M:%S)] $*" >&2; }
log_error() { echo "[$(date +%H:%M:%S)] ERROR: $*" >&2; }

check_status() {   # $1 label, $2 command, remaining arguments are command arguments
  local label="$1"
  shift
  if "$@"; then
    echo "PASS $label"
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    if [ -n "$STOP_REASON" ]; then
      echo "STOP $label"
      return 0
    fi
    echo "FAIL $label"
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi
}

trap 'for p in "${NODE_PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done' EXIT

remove_node_pid() {
  local target="$1" p kept=()
  for p in "${NODE_PIDS[@]:-}"; do
    [ "$p" = "$target" ] || kept+=("$p")
  done
  NODE_PIDS=("${kept[@]}")
}

port_is_free() { ! (exec 3<>"/dev/tcp/127.0.0.1/$BACKEND_PORT") 2>/dev/null; }
binary_exists() { [ -f "$BINARY" ]; }

preflight() {
  case "$MODE" in
    full|baseline) ;;
    *) log_error "Invalid MODE value: $MODE"; exit 2 ;;
  esac
  case "$LEGS" in
    A|B|AB) ;;
    *) log_error "Invalid LEGS value: $LEGS"; exit 2 ;;
  esac
  if ! port_is_free; then log_error "Port $BACKEND_PORT is already in use"; exit 2; fi
  if ! command -v sqlite3 >/dev/null 2>&1; then log_error "sqlite3 not found in PATH"; exit 2; fi
  if ! command -v curl >/dev/null 2>&1; then log_error "curl not found in PATH"; exit 2; fi
  if ! command -v git >/dev/null 2>&1; then log_error "git not found in PATH"; exit 2; fi
  if ! command -v setsid >/dev/null 2>&1; then log_error "setsid not found in PATH"; exit 2; fi
  if ! binary_exists; then
    log_error "Binary not found: $BINARY"
    log_error "Build with: cargo build --release -p server --bin vks-node-server"
    exit 2
  fi
  if [ -z "$SCRATCH_ROOT" ]; then SCRATCH_ROOT=$(mktemp -d /tmp/wal-repro.XXXXXX); else mkdir -p "$SCRATCH_ROOT"; fi
  log_info "Using scratch directory: $SCRATCH_ROOT"
}

run_node() {
  local legdir="$1"
  shift
  local extra_env=()
  local env_var
  extra_env=("$@")
  mkdir -p "$legdir"/{backup,worktrees,logs}
  (
    export HOST=0.0.0.0 BACKEND_PORT=$BACKEND_PORT
    export VK_ASSET_DIR="$legdir" VK_DATABASE_PATH="$legdir/db.sqlite"
    export VK_BACKUP_DIR="$legdir/backup" VK_WORKTREE_DIR="$legdir/worktrees" VK_LOG_DIR="$legdir/logs"
    for env_var in "${extra_env[@]}"; do export "$env_var"; done
    exec "$BINARY" >"$legdir/node.log" 2>&1
  ) &
  RUN_NODE_PID=$!
  NODE_PIDS+=("$RUN_NODE_PID")
}

wait_health() {
  local response start_ns now_ns elapsed_ms start_seconds elapsed_seconds remaining_seconds
  start_ns=$(date +%s%N)
  start_seconds=$SECONDS
  while :; do
    elapsed_seconds=$((SECONDS - start_seconds))
    if (( elapsed_seconds >= 30 )); then break; fi
    remaining_seconds=$((30 - elapsed_seconds))
    response=$(curl -s --connect-timeout 2 --max-time "$remaining_seconds" "http://127.0.0.1:$BACKEND_PORT/api/health" 2>/dev/null || true)
    if printf '%s' "$response" | grep -q '"database_ready":true'; then
      now_ns=$(date +%s%N)
      elapsed_ms=$(( (now_ns - start_ns) / 1000000 ))
      log_info "Health check passed after ${elapsed_ms}ms"
      return 0
    fi
    sleep 0.5
  done
  log_error "Health check timeout after 30s"
  return 1
}

stop_node() {
  local pid="$1" start_seconds
  if ! kill -0 "$pid" 2>/dev/null; then
    log_error "Node $pid was already dead before graceful stop"
    wait "$pid" 2>/dev/null || true
    remove_node_pid "$pid"
    return 1
  fi
  if ! kill "$pid" 2>/dev/null; then
    log_error "Could not send SIGTERM to node $pid"
    if ! kill -0 "$pid" 2>/dev/null; then
      wait "$pid" 2>/dev/null || true
      remove_node_pid "$pid"
      return 1
    fi
    log_error "Node $pid remained alive after SIGTERM send failure; sending SIGKILL"
    kill -KILL "$pid" 2>/dev/null || true
    start_seconds=$SECONDS
    while kill -0 "$pid" 2>/dev/null; do
      if (( SECONDS - start_seconds >= 15 )); then
        log_error "Node $pid remained alive after SIGKILL"
        return 1
      fi
      sleep 0.1
    done
    wait "$pid" 2>/dev/null || true
    remove_node_pid "$pid"
    return 1
  fi
  start_seconds=$SECONDS
  while kill -0 "$pid" 2>/dev/null; do
    if (( SECONDS - start_seconds >= 15 )); then
      log_error "Node $pid ignored SIGTERM; sending SIGKILL"
      kill -KILL "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      remove_node_pid "$pid"
      return 1
    fi
    sleep 0.1
  done
  wait "$pid" 2>/dev/null || true
  remove_node_pid "$pid"
}

seed_session() {
  local legdir="$1" raw hash id hive timestamp_ms
  raw=$(head -c 32 /dev/urandom | sha256sum | cut -d' ' -f1) || return 1
  hash=$(printf '%s' "$raw" | sha256sum | cut -d' ' -f1) || return 1
  id=$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n') || return 1
  hive=$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n') || return 1
  timestamp_ms=$(date +%s%3N) || return 1
  sqlite3 "$legdir/db.sqlite" "INSERT INTO browser_sessions (id, token_hash, hive_user_id, created_at, revoked_at) VALUES (X'$id', '$hash', X'$hive', $timestamp_ms, NULL);" || return 1
  printf '%s' "$raw" >"$legdir/.cookie_raw" || return 1
}

api_call() {
  local method="$1" path="$2" legdir="$3" body="${4:-}" cookie_raw response http_code response_body
  cookie_raw=$(<"$legdir/.cookie_raw") || return 1
  if [ -z "$body" ]; then
    if ! response=$(curl -s --connect-timeout 2 --max-time 10 -w '\n%{http_code}' -X "$method" -H "Cookie: vks_browser_session=$cookie_raw" "http://127.0.0.1:$BACKEND_PORT$path" 2>/dev/null); then
      printf '000\ntransport failure calling %s %s\n' "$method" "$path"
      return 0
    fi
  else
    if ! response=$(curl -s --connect-timeout 2 --max-time 10 -w '\n%{http_code}' -X "$method" -H "Cookie: vks_browser_session=$cookie_raw" -H 'Content-Type: application/json' -d "$body" "http://127.0.0.1:$BACKEND_PORT$path" 2>/dev/null); then
      printf '000\ntransport failure calling %s %s\n' "$method" "$path"
      return 0
    fi
  fi
  http_code=$(printf '%s\n' "$response" | tail -n 1)
  response_body=$(printf '%s\n' "$response" | sed '$d')
  printf '%s\n%s\n' "$http_code" "$response_body"
}

auth_drift_stop() {
  local http_code="$1" path="$2"
  if [ "$http_code" = 401 ] || [ "$http_code" = 403 ]; then
    STOP_REASON="seeded session rejected by $path with HTTP $http_code"
    log_error "STOP: auth drift: $STOP_REASON"
    return 0
  fi
  return 1
}

api_write() {
  local legdir="$1" marker="$2" project_id body response http_code resp_body task_id
  project_id=$(<"$legdir/.project_id")
  body=$(printf '{"project_id":"%s","title":"%s"}' "$project_id" "$marker")
  response=$(api_call POST /api/tasks "$legdir" "$body")
  http_code=$(printf '%s\n' "$response" | head -n 1)
  resp_body=$(printf '%s\n' "$response" | tail -n 1)
  if auth_drift_stop "$http_code" /api/tasks; then return 1; fi
  [ "$http_code" = 200 ] || { log_error "api_write failed: HTTP $http_code"; return 1; }
  printf '%s' "$resp_body" | grep -q '"success":true' || { log_error 'api_write returned .success==false'; return 1; }
  task_id=$(printf '%s' "$resp_body" | grep -o '"id":"[^"]*"' | head -n 1 | cut -d'"' -f4)
  [ -n "$task_id" ] || { log_error 'api_write returned an empty task id'; return 1; }
  printf '%s\n' "$task_id" >&2
}

create_project() {
  local legdir="$1" name="$2" repo_dir project_body response http_code resp_body project_id
  repo_dir="$legdir/repo"
  git init -q "$repo_dir" || return 1
  project_body=$(printf '{"name":"%s","git_repo_path":"%s","use_existing_repo":true}' "$name" "$repo_dir")
  response=$(api_call POST /api/projects "$legdir" "$project_body")
  http_code=$(printf '%s\n' "$response" | head -n 1)
  resp_body=$(printf '%s\n' "$response" | tail -n 1)
  if auth_drift_stop "$http_code" /api/projects; then return 1; fi
  if [ "$http_code" != 200 ] || ! printf '%s' "$resp_body" | grep -q '"success":true'; then
    log_error "create_project failed: HTTP $http_code body=$resp_body"
    return 1
  fi
  project_id=$(printf '%s' "$resp_body" | grep -o '"id":"[^"]*"' | head -n 1 | cut -d'"' -f4)
  [ -n "$project_id" ] || return 1
  printf '%s' "$project_id" >"$legdir/.project_id"
}

trip_detector() {
  local pid="$1" start_ns now_ns elapsed_ms evidence start_seconds
  TRIP_FIRED=0
  start_ns=$(date +%s%N)
  start_seconds=$SECONDS
  while (( SECONDS - start_seconds < 30 )); do
    if [ ! -d "/proc/$pid/fd" ]; then return 1; fi
    evidence=$(ls -l "/proc/$pid/fd" 2>/dev/null | grep 'db.sqlite-wal (deleted)' || true)
    if [ -n "$evidence" ]; then
      now_ns=$(date +%s%N)
      elapsed_ms=$(( (now_ns - start_ns) / 1000000 ))
      TRIP_FIRED=1
      log_info "WAL evidence after ${elapsed_ms}ms: $evidence"
      return 0
    fi
    sleep 0.5
  done
  now_ns=$(date +%s%N)
  elapsed_ms=$(( (now_ns - start_ns) / 1000000 ))
  log_info "Trip detector timeout after ${elapsed_ms}ms"
  return 1
}

refusal_latch_detector() {
  local legdir="$1" start_ns now_ns elapsed_ms start_seconds
  start_ns=$(date +%s%N)
  start_seconds=$SECONDS
  while (( SECONDS - start_seconds < 30 )); do
    if grep -q 'wal_write_refusal_active' "$legdir/node.log"; then
      now_ns=$(date +%s%N)
      elapsed_ms=$(( (now_ns - start_ns) / 1000000 ))
      log_info "Refusal latch evidence after ${elapsed_ms}ms"
      return 0
    fi
    sleep 0.5
  done
  now_ns=$(date +%s%N)
  elapsed_ms=$(( (now_ns - start_ns) / 1000000 ))
  log_info "Refusal latch detector timeout after ${elapsed_ms}ms"
  return 1
}

external_write_session() {
  local legdir="$1" success_sentinel session_pid start_seconds
  success_sentinel="$legdir/.write-session-succeeded"
  WRITE_SESSION_SUCCEEDED=0
  rm -f "$success_sentinel"
  setsid sqlite3 "$legdir/db.sqlite" "PRAGMA user_version=$RANDOM;" >/dev/null 2>&1 &
  session_pid=$!
  start_seconds=$SECONDS
  while kill -0 "$session_pid" 2>/dev/null; do
    if (( SECONDS - start_seconds >= 30 )); then
      log_error "External write session $session_pid timed out; terminating process group"
      kill -TERM -- "-$session_pid" 2>/dev/null || true
      sleep 0.1
      if kill -0 "$session_pid" 2>/dev/null; then kill -KILL -- "-$session_pid" 2>/dev/null || true; fi
      wait "$session_pid" 2>/dev/null || true
      return 1
    fi
    sleep 0.1
  done
  if wait "$session_pid" 2>/dev/null; then : >"$success_sentinel"; fi
  [ -f "$success_sentinel" ] && WRITE_SESSION_SUCCEEDED=1
  [ "$WRITE_SESSION_SUCCEEDED" = 1 ]
}

record_timing() {
  local legdir="$1" marker="$2" timing_file="$3" start_ns end_ns
  start_ns=$(date +%s%N)
  if api_write "$legdir" "$marker" >/dev/null; then
    end_ns=$(date +%s%N)
    printf 'write_latency_ms=%s\n' "$(( (end_ns - start_ns) / 1000000 ))" >>"$timing_file"
    return 0
  fi
  return 1
}

boot_and_seed() {
  local legdir="$1" guard="$2" label="$3" pid
  local before=$FAIL_COUNT
  run_node "$legdir"
  pid=$RUN_NODE_PID
  check_status "$label boot 1 is healthy" wait_health "$legdir"
  if [ "$FAIL_COUNT" -ne "$before" ]; then stop_node "$pid" || true; return 1; fi
  check_status "$label boot 1 stopped gracefully" stop_node "$pid"
  if [ "$FAIL_COUNT" -ne "$before" ]; then return 1; fi
  check_status "$label browser session seeded" seed_session "$legdir"
  if [ "$FAIL_COUNT" -ne "$before" ]; then return 1; fi
  run_node "$legdir" "VK_WAL_GUARD=$guard"
  pid=$RUN_NODE_PID
  check_status "$label boot 2 is healthy" wait_health "$legdir"
  if [ "$FAIL_COUNT" -ne "$before" ]; then stop_node "$pid" || true; return 1; fi
  BOOT_PID=$RUN_NODE_PID
  [ "$FAIL_COUNT" -eq "$before" ]
}

run_leg_a() {
  local legdir="$1" pid count journal_mode i setup_before
  local before=$FAIL_COUNT before_pass=$PASS_COUNT
  LEG_COMPLETION_SENTINELS[A]="$legdir/.completed"
  echo ""; echo "========== LEG A: guard-on durability =========="; echo "MODE=$MODE"
  if ! boot_and_seed "$legdir" on 'Leg A'; then return 1; fi
  pid=$BOOT_PID
  setup_before=$FAIL_COUNT
  check_status 'Leg A project created' create_project "$legdir" repro-A
  if [ -n "$STOP_REASON" ]; then stop_node "$pid" || true; return 1; fi
  if [ ! -f "$legdir/.project_id" ]; then
    if [ "$FAIL_COUNT" -eq "$setup_before" ]; then
      check_status 'Leg A project id was captured' false
    fi
    check_status 'Leg A stopped after setup failure' stop_node "$pid"
    return 1
  fi
  if [ "$MODE" = full ]; then
    check_status 'Leg A marker-A-pre written' api_write "$legdir" marker-A-pre
    if [ -n "$STOP_REASON" ]; then stop_node "$pid" || true; return 1; fi
    check_status 'Leg A external write session executed' external_write_session "$legdir"
    if trip_detector "$pid"; then
      check_status 'Leg A no external WAL unlink (detector timed out)' false
    else
      check_status 'Leg A no external WAL unlink (detector timed out)' true
    fi
    for i in 1 2 3 4 5; do
      check_status "Leg A timing-$i written" record_timing "$legdir" "timing-$i" "$SCRATCH_ROOT/timings.txt"
      if [ -n "$STOP_REASON" ]; then stop_node "$pid" || true; return 1; fi
    done
    check_status 'Leg A marker-A-post API write' api_write "$legdir" marker-A-post
    if [ -n "$STOP_REASON" ]; then stop_node "$pid" || true; return 1; fi
    check_status 'Leg A stopped gracefully' stop_node "$pid"
    count=$(sqlite3 "$legdir/db.sqlite" "SELECT count(*) FROM tasks WHERE title='marker-A-post';")
    check_status 'Leg A marker-A-post persisted' test "$count" = 1
    journal_mode=$(sqlite3 "$legdir/db.sqlite" 'PRAGMA journal_mode;')
    check_status 'Leg A journal_mode is wal' test "$journal_mode" = wal
  else
    for i in 1 2 3 4 5; do
      check_status "Leg A timing-$i written" record_timing "$legdir" "timing-$i" "$SCRATCH_ROOT/timings.txt"
      if [ -n "$STOP_REASON" ]; then stop_node "$pid" || true; return 1; fi
    done
    check_status 'Leg A stopped gracefully' stop_node "$pid"
    journal_mode=$(sqlite3 "$legdir/db.sqlite" 'PRAGMA journal_mode;')
    check_status 'Leg A journal_mode is wal' test "$journal_mode" = wal
  fi
  LEG_PASS_COUNTS[A]=$((PASS_COUNT - before_pass))
  LEG_FAIL_COUNTS[A]=$((FAIL_COUNT - before))
  LEG_TOTAL_COUNTS[A]=$((LEG_PASS_COUNTS[A] + LEG_FAIL_COUNTS[A]))
  if [ "$FAIL_COUNT" -eq "$before" ]; then LEG_RESULTS[A]=PASS; else LEG_RESULTS[A]=FAIL; fi
  : >"${LEG_COMPLETION_SENTINELS[A]}"
  [ "${LEG_RESULTS[A]}" = PASS ]
}

run_leg_b_attempt() {
  local legdir="$1" attempt="$2" completion_sentinel="$3" pid project_id response http_code resp_body count journal_mode i setup_before completion_allowed=1 trip_detected=0
  local before=$FAIL_COUNT
  mkdir -p "$legdir"
  if ! boot_and_seed "$legdir" off "Leg B attempt $attempt"; then return 1; fi
  pid=$BOOT_PID
  setup_before=$FAIL_COUNT
  check_status "Leg B attempt $attempt project created" create_project "$legdir" "repro-B-$attempt"
  if [ -n "$STOP_REASON" ]; then stop_node "$pid" || true; return 1; fi
  if [ ! -f "$legdir/.project_id" ]; then
    if [ "$FAIL_COUNT" -eq "$setup_before" ]; then
      check_status "Leg B attempt $attempt project id was captured" false
    fi
    check_status "Leg B attempt $attempt stopped after setup failure" stop_node "$pid"
    return 1
  fi
  project_id=$(<"$legdir/.project_id")
  if [ "$MODE" = full ]; then
    check_status "Leg B attempt $attempt marker-B-pre written" api_write "$legdir" marker-B-pre
    if [ -n "$STOP_REASON" ]; then stop_node "$pid" || true; return 1; fi
    rm -f "$legdir/db.sqlite-wal" "$legdir/db.sqlite-shm"
    if trip_detector "$pid"; then
      trip_detected=1
    fi
    check_status "Leg B attempt $attempt fault injection removed WAL and SHM" bash -c '[ ! -e "$1" ] && [ ! -e "$2" ]' _ "$legdir/db.sqlite-wal" "$legdir/db.sqlite-shm"
    if [ "$trip_detected" -eq 1 ]; then
      check_status "Leg B attempt $attempt detected external WAL unlink" true
    elif [ "$attempt" = 1 ]; then
      log_info 'Leg B attempt 1 detector timeout is provisional; retrying on fresh scratch database'
      check_status "Leg B attempt $attempt stopped after provisional detector timeout" stop_node "$pid"
      if [ "$FAIL_COUNT" -eq "$before" ]; then
        : >"$completion_sentinel" || return 1
        return 0
      fi
      completion_allowed=0
      check_status "Leg B attempt $attempt detected external WAL unlink" false
    else
      check_status "Leg B attempt $attempt detected external WAL unlink" false
    fi
    check_status "Leg B attempt $attempt wal_unlinked_externally logged" grep -q 'wal_unlinked_externally' "$legdir/node.log"
    check_status "Leg B attempt $attempt unlink log names db path" bash -c "grep 'wal_unlinked_externally' \"\$1\" | grep -qF \"\$2\"" _ "$legdir/node.log" "$legdir/db.sqlite"
    check_status "Leg B attempt $attempt wal_write_refusal_active latched" refusal_latch_detector "$legdir"
    response=$(api_call POST /api/tasks "$legdir" "{\"project_id\":\"$project_id\",\"title\":\"marker-B-post\"}")
    http_code=$(printf '%s\n' "$response" | head -n 1); resp_body=$(printf '%s\n' "$response" | tail -n 1)
    if auth_drift_stop "$http_code" /api/tasks; then stop_node "$pid" || true; return 1; fi
    if [ "$http_code" = 000 ] || ! [[ "$http_code" =~ ^[1-5][0-9]{2}$ ]]; then
      check_status "Leg B attempt $attempt marker-B-post rejected (no HTTP response: $resp_body)" false
    elif ! [[ "$http_code" =~ ^2[0-9]{2}$ ]] || printf '%s' "$resp_body" | grep -Eq '"success"[[:space:]]*:[[:space:]]*false'; then
      check_status "Leg B attempt $attempt marker-B-post rejected" true
    else
      check_status "Leg B attempt $attempt marker-B-post rejected (HTTP $http_code, success was not false)" false
    fi
    check_status "Leg B attempt $attempt node remains alive after refusal" kill -0 "$pid"
    check_status "Leg B attempt $attempt stopped gracefully" stop_node "$pid"
    count=$(sqlite3 "$legdir/db.sqlite" "SELECT count(*) FROM tasks WHERE title='marker-B-post';")
    check_status "Leg B attempt $attempt marker-B-post was not persisted" test "$count" = 0
  else
    for i in 1 2 3 4 5; do
      check_status "Leg B attempt $attempt timing-$i written" record_timing "$legdir" "timing-$i" "$SCRATCH_ROOT/timings-B.txt"
      if [ -n "$STOP_REASON" ]; then stop_node "$pid" || true; return 1; fi
    done
    check_status "Leg B attempt $attempt stopped gracefully" stop_node "$pid"
    journal_mode=$(sqlite3 "$legdir/db.sqlite" 'PRAGMA journal_mode;')
    check_status "Leg B attempt $attempt journal_mode is wal" test "$journal_mode" = wal
  fi
  if [ "$completion_allowed" -eq 1 ]; then : >"$completion_sentinel" || return 1; fi
  [ "$FAIL_COUNT" -eq "$before" ]
}

run_leg_b() {
  local legdir="$1" retry_legdir attempt_sentinel final_sentinel
  local before=$FAIL_COUNT before_pass=$PASS_COUNT
  final_sentinel="$legdir/.completed"
  LEG_COMPLETION_SENTINELS[B]="$final_sentinel"
  echo ""; echo "========== LEG B: guard-off detection+refusal =========="; echo "MODE=$MODE"
  attempt_sentinel="$legdir/.attempt-1.completed"
  run_leg_b_attempt "$legdir" 1 "$attempt_sentinel" || true
  if [ ! -f "$attempt_sentinel" ]; then return 1; fi
  if [ "$MODE" = full ] && [ "$TRIP_FIRED" -eq 0 ] && [ "$FAIL_COUNT" -eq "$before" ]; then
    log_info 'Leg B retry: full sequence on fresh scratch database'
    PASS_COUNT=$before_pass
    retry_legdir=$(mktemp -d "$SCRATCH_ROOT/leg-b-retry.XXXXXX") || return 1
    attempt_sentinel="$retry_legdir/.attempt-2.completed"
    run_leg_b_attempt "$retry_legdir" 2 "$attempt_sentinel" || true
    if [ ! -f "$attempt_sentinel" ]; then return 1; fi
  fi
  if [ "$FAIL_COUNT" -eq "$before" ]; then LEG_RESULTS[B]=PASS; else LEG_RESULTS[B]=FAIL; fi
  LEG_PASS_COUNTS[B]=$((PASS_COUNT - before_pass))
  LEG_FAIL_COUNTS[B]=$((FAIL_COUNT - before))
  LEG_TOTAL_COUNTS[B]=$((LEG_PASS_COUNTS[B] + LEG_FAIL_COUNTS[B]))
  : >"$final_sentinel" || return 1
  [ "${LEG_RESULTS[B]}" = PASS ]
}

record_aborted_leg() {
  local leg="$1" before_pass="$2" before_fail="$3"
  if [ ! -f "${LEG_COMPLETION_SENTINELS[$leg]}" ]; then
    LEG_RESULTS["$leg"]=ABORTED
    LEG_PASS_COUNTS["$leg"]=$((PASS_COUNT - before_pass))
    LEG_FAIL_COUNTS["$leg"]=$((FAIL_COUNT - before_fail))
    LEG_TOTAL_COUNTS["$leg"]=$((LEG_PASS_COUNTS["$leg"] + LEG_FAIL_COUNTS["$leg"]))
  fi
}

record_stopped_leg() {
  local leg="$1"
  LEG_RESULTS["$leg"]=SKIPPED_DUE_TO_STOP
  LEG_PASS_COUNTS["$leg"]=0
  LEG_FAIL_COUNTS["$leg"]=0
  LEG_TOTAL_COUNTS["$leg"]=0
}

main() {
  preflight
  : >"$SCRATCH_ROOT/timings.txt"
  : >"$SCRATCH_ROOT/timings-B.txt"
  local leg_a_dir leg_b_dir a_pass a_fail b_pass b_fail
  case "$LEGS" in
    A)
      leg_a_dir=$(mktemp -d "$SCRATCH_ROOT/leg-a.XXXXXX")
      a_pass=$PASS_COUNT; a_fail=$FAIL_COUNT
      run_leg_a "$leg_a_dir" || true
      record_aborted_leg A "$a_pass" "$a_fail"
      ;;
    B)
      leg_b_dir=$(mktemp -d "$SCRATCH_ROOT/leg-b.XXXXXX")
      b_pass=$PASS_COUNT; b_fail=$FAIL_COUNT
      run_leg_b "$leg_b_dir" || true
      record_aborted_leg B "$b_pass" "$b_fail"
      ;;
    AB)
      leg_a_dir=$(mktemp -d "$SCRATCH_ROOT/leg-a.XXXXXX")
      a_pass=$PASS_COUNT; a_fail=$FAIL_COUNT
      run_leg_a "$leg_a_dir" || true
      record_aborted_leg A "$a_pass" "$a_fail"
      if [ -n "$STOP_REASON" ]; then
        record_stopped_leg B
      else
        leg_b_dir=$(mktemp -d "$SCRATCH_ROOT/leg-b.XXXXXX")
        b_pass=$PASS_COUNT; b_fail=$FAIL_COUNT
        run_leg_b "$leg_b_dir" || true
        record_aborted_leg B "$b_pass" "$b_fail"
      fi
      ;;
  esac
  echo ""; echo "========== SUMMARY =========="; echo "Total PASS: $PASS_COUNT"; echo "Total FAIL: $FAIL_COUNT"; echo ""; echo 'Leg results:'
  for leg in A B; do
    if [[ "$LEGS" == *"$leg"* ]]; then
      echo "  LEG $leg: ${LEG_RESULTS[$leg]:-UNKNOWN} PASS=${LEG_PASS_COUNTS[$leg]:-0} FAIL=${LEG_FAIL_COUNTS[$leg]:-0} TOTAL=${LEG_TOTAL_COUNTS[$leg]:-0}"
    fi
  done
  if [ -n "$STOP_REASON" ]; then
    echo ''; log_error "STOP: $STOP_REASON"; exit 1
  fi
  for leg in A B; do
    if [[ "$LEGS" == *"$leg"* ]] && [[ "${LEG_RESULTS[$leg]:-UNKNOWN}" = ABORTED || "${LEG_RESULTS[$leg]:-UNKNOWN}" = UNKNOWN ]]; then
      echo ''; log_error 'One or more selected legs did not complete'; exit 1
    fi
  done
  if [ "$FAIL_COUNT" -gt 0 ]; then echo ''; log_error 'One or more assertions failed'; exit 1; fi
  log_info 'All tests passed'
}

main "$@"
