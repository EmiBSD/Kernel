#!/bin/bash
#
# Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
#
# Permission to use, copy, modify, and distribute this software for any
# purpose with or without fee is hereby granted, provided that the above
# copyright notice and this permission notice appear in all copies.
#
# THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
# WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
# MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
# ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
# WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
# ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
# OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
#
# PreToolUse hook (Bash): before a `git commit`, run `cargo xtask lz check`
# and `cargo fmt --all -- --check` in the checkout the command runs in (the
# main checkout or an agent's worktree). Exit 2 blocks the commit and hands
# the failure to the model; anything that is not a commit passes at once.
# Clippy and the smokes stay in `just ci` (too slow for every commit).
payload=$(cat)
cmd=$(printf '%s' "$payload" | jq -r '.tool_input.command // empty')
# A `git commit` at the start or after a separator, with any `VAR=value` assignments or
# `env` before `git` (`PATH=... git commit`), and `git -C <dir> commit`.
re='(^|[;&|(]|&&)[[:space:]]*((env|[A-Za-z_][A-Za-z0-9_]*=[^[:space:]]*)[[:space:]]+)*git([[:space:]]+-C[[:space:]]+[^[:space:]]+)?[[:space:]]+commit([[:space:]]|$)'
printf '%s\n' "$cmd" | grep -qE "$re" || exit 0
# The checkout the commit runs in: `git -C <dir>` first, then a leading `cd <dir> &&` or
# `cd <dir>;` (a literal path; an unexpanded `$var` is ignored), then the tool's cwd.
dir=$(printf '%s\n' "$cmd" | grep -oE 'git[[:space:]]+-C[[:space:]]+[^[:space:]]+' | head -1 | awk '{print $3}')
[ -n "$dir" ] || dir=$(printf '%s\n' "$cmd" | sed -nE 's/^[[:space:]]*cd[[:space:]]+"?([^"[:space:];&|]+)"?[[:space:]]*(&&|;).*/\1/p' | head -1)
case "$dir" in *'$'*) dir= ;; esac
dir=${dir//\"/}
cwd=$(printf '%s' "$payload" | jq -r '.cwd // empty')
{ [ -n "$dir" ] && cd "$dir" 2>/dev/null; } || cd "${cwd:-${CLAUDE_PROJECT_DIR:-.}}" 2>/dev/null || exit 0
cd "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || exit 0
[ -f lineage.toml ] || exit 0
if [ -n "$PRE_COMMIT_CHECK_DRY" ]; then printf 'pre-commit-check: would check in %s\n' "$PWD"; exit 0; fi
export PATH=/opt/homebrew/opt/rustup/bin:$PATH
if ! out=$(cargo xtask lz check 2>&1); then
    printf 'commit blocked by .claude/hooks/pre-commit-check.sh: `cargo xtask lz check` failed in %s\n%s\n' "$PWD" "$(printf '%s\n' "$out" | grep -v '^ *\(Compiling\|Finished\|Running\)' | tail -30)" >&2
    exit 2
fi
if ! out=$(cargo fmt --all -- --check 2>&1); then
    printf 'commit blocked by .claude/hooks/pre-commit-check.sh: `cargo fmt --all -- --check` failed in %s (run `cargo fmt --all`)\n%s\n' "$PWD" "$(printf '%s\n' "$out" | head -40)" >&2
    exit 2
fi
exit 0
