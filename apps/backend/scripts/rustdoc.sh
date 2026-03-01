#!/usr/bin/env bash
set -euo pipefail

exec rustup run 1.91.0 rustdoc "$@"
