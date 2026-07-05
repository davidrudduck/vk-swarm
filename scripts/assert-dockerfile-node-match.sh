#!/usr/bin/env bash
# Assert the two fe-builder stages in Dockerfile and crates/remote/Dockerfile
# are on the same node base image major version. Catches drift between the two
# Dockerfiles (the bug that caused the ERR_UNKNOWN_BUILTIN_MODULE failure).
# Usage: bash scripts/assert-dockerfile-node-match.sh
# Exit 0 if they match, 1 with a diff if they don't.
set -euo pipefail

ROOT_DOCKERFILE="${1:-Dockerfile}"
REMOTE_DOCKERFILE="${2:-crates/remote/Dockerfile}"

ROOT_NODE=$(grep -oE 'FROM node:[0-9]+-alpine AS builder' "$ROOT_DOCKERFILE" | sed 's/FROM //;s/ AS builder//' || true)
REMOTE_NODE=$(grep -oE 'FROM node:[0-9]+-alpine AS fe-builder' "$REMOTE_DOCKERFILE" | sed 's/FROM //;s/ AS fe-builder//' || true)

if [ -z "$ROOT_NODE" ]; then
    echo "ERROR: could not find 'FROM node:<N>-alpine AS builder' in $ROOT_DOCKERFILE" >&2
    exit 2
fi

if [ -z "$REMOTE_NODE" ]; then
    echo "ERROR: could not find 'FROM node:<N>-alpine AS fe-builder' in $REMOTE_DOCKERFILE" >&2
    exit 2
fi

if [ "$ROOT_NODE" != "$REMOTE_NODE" ]; then
    echo "MISMATCH: root Dockerfile uses $ROOT_NODE but crates/remote/Dockerfile uses $REMOTE_NODE" >&2
    echo "Fix: update the older one to match." >&2
    exit 1
fi

echo "OK: both Dockerfiles use ${ROOT_NODE}"
exit 0