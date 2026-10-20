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
# PreToolUse hook of the writing subagents (redesigner, mechanical, integrator,
# debugger, milestone-coordinator; the `hooks:` field of their frontmatter). It
# makes mechanical what `.claude/rules/subagents.md` and `reference-readonly.md`
# say in words: a subagent never pushes, never amends, never changes the
# reference trees (`reference/`: the OpenBSD and LZ clones) or the LZ pin
# (`lz/`). Exit 2 refuses the call and hands the reason to the agent; anything
# else exits 0 at once. The main session has no such hook: pushing and the pin
# bump are the user's, done from there when the user asks. It is a belt
# against a slip, not a wall against an adversary: a command built at run time
# (`eval`, a script the agent writes and then runs) is beyond a pattern.
payload=$(cat)
tool=$(printf '%s' "$payload" | jq -r '.tool_name // empty')
cwd=$(printf '%s' "$payload" | jq -r '.cwd // empty')

refuse() {
    printf 'blocked by .claude/hooks/agent-guard.sh: %s (.claude/rules/subagents.md, reference-readonly.md; the main session does this when the user asks)\n' "$1" >&2
    exit 2
}

# The checkout the agent runs in, to make a path repository-relative.
root=$(cd "${cwd:-.}" 2>/dev/null && git rev-parse --show-toplevel 2>/dev/null)
# True when a path (absolute or relative to the cwd) is under `reference/` or `lz/` of the
# checkout, or names the main checkout's `reference/` through a worktree's symlink.
guarded_path() {
    local p=$1 rel
    case "$p" in /*) ;; *) p="${cwd:-.}/$p" ;; esac
    rel=${p#"$root"/}
    case "$rel" in reference/*|reference|lz/*|lz) return 0 ;; esac
    case "$p" in */reference/*|*/reference) return 0 ;; esac
    return 1
}

case "$tool" in
Edit|Write|NotebookEdit)
    path=$(printf '%s' "$payload" | jq -r '.tool_input.file_path // .tool_input.notebook_path // empty')
    [ -n "$path" ] || exit 0
    guarded_path "$path" && refuse "a subagent never writes under reference/ or lz/ ($path)"
    exit 0
    ;;
Bash) ;;
*) exit 0 ;;
esac

cmd=$(printf '%s' "$payload" | jq -r '.tool_input.command // empty')
[ -n "$cmd" ] || exit 0
# `git`, with any `VAR=x` or `env` before it and an optional `-C <dir>`, as a shell word at
# the start of a command or after a separator.
G='(^|[;&|(`]|&&)[[:space:]]*((env|[A-Za-z_][A-Za-z0-9_]*=[^[:space:]]*)[[:space:]]+)*git([[:space:]]+-C[[:space:]]+[^[:space:]]+)?[[:space:]]+'

printf '%s\n' "$cmd" | grep -qE "${G}push([[:space:]]|$)" && refuse "a subagent never pushes"
printf '%s\n' "$cmd" | grep -qE "${G}commit[^;&|]*[[:space:]]--amend([[:space:]]|$)" && refuse "a subagent never amends a commit"
printf '%s\n' "$cmd" | grep -qE "${G}(push|fetch|pull)[^;&|]*[[:space:]]-f([[:space:]]|$)|[[:space:]]--force([[:space:]]|$)" && \
    printf '%s\n' "$cmd" | grep -qE "${G}push" && refuse "a subagent never force-pushes"

# `cd` into a reference tree (the rule of 2026-10-10: read it with `git show`, never from inside).
printf '%s\n' "$cmd" | grep -qE '(^|[;&|(]|&&)[[:space:]]*cd[[:space:]]+"?[^;&|[:space:]"]*reference(/|"|[[:space:]]|$)' && \
    refuse "a subagent never cds into reference/"

# `git -C <reference tree> <subcommand>`: only reading subcommands (and `fetch`, the LZ sync's
# one network use) are allowed there; a checkout, reset, pull, sparse-checkout change or any
# write moves the tree off the pin, which only the user does.
while read -r sub; do
    case "$sub" in
    fetch|show|log|diff|grep|blame|ls-files|ls-tree|rev-parse|cat-file|status|describe|format-patch|name-rev|rev-list|shortlog|merge-base|remote|branch|tag|config|var|count-objects|verify-pack|whatchanged) ;;
    *) refuse "a subagent never runs 'git $sub' in a reference tree" ;;
    esac
done < <(printf '%s\n' "$cmd" | grep -oE 'git[[:space:]]+-C[[:space:]]+"?[^[:space:]"]*reference(/[^[:space:]"]*)?"?[[:space:]]+[a-z-]+' | awk '{print $NF}')
# `branch`/`tag`/`remote`/`config` with a mutating flag inside a reference tree:
printf '%s\n' "$cmd" | grep -qE 'git[[:space:]]+-C[[:space:]]+"?[^[:space:]"]*reference/[^[:space:]"]*"?[[:space:]]+(branch|tag|remote|config|sparse-checkout)[[:space:]]+(-[dDmMf]|--delete|--move|--force|add|set|remove|rename|rm|init|disable|reapply|--add|--unset|--replace-all|[a-z]+\.[a-zA-Z.]+[[:space:]]+[^[:space:]])' && \
    refuse "a subagent never changes a reference tree's refs or configuration"

# The one write a worktree agent makes under `reference/` (`subagents.md`, "Setup in a
# worktree"): the symlinks `reference/openbsd-src` and `reference/emibsd-lz` to the main
# checkout's clones, made with `ln -s` and unlinked before the worktree goes. Only the link
# itself: a path inside a clone, a trailing slash (which follows the link) or a recursive
# flag is a write into the tree and stays refused. Those words are replaced by `true`
# before the generic check below.
L='reference/(openbsd-src|emibsd-lz)'
# A space before every separator first, so a link name right before `;`, `&&` or `|` ends
# in whitespace like any other word (every pattern here allows that space).
cmd=$(printf '%s\n' "$cmd" | sed -E 's/([;&|]+)/ \1/g' | sed -E \
    -e "s#(^|[;&|(]|&&)([[:space:]]*)ln[[:space:]]+-s[fn]*[[:space:]]+\"?[^[:space:]\"]+\"?[[:space:]]+\"?$L\"?([[:space:]]|\$)#\\1\\2true\\4#g" \
    -e "s#(^|[;&|(]|&&)([[:space:]]*)(rm|unlink)[[:space:]]+(-f[[:space:]]+)?(\"?$L\"?([[:space:]]+|\$))+#\\1\\2true #g")

# Any other write that names a guarded path: a write verb or a redirection together with a
# `reference/` or `lz/` path word in the same command.
if printf '%s\n' "$cmd" | grep -qE '(^|[[:space:]"'"'"'=/])(reference|lz)/'; then
    W='(^|[;&|(`]|&&)[[:space:]]*((env|[A-Za-z_][A-Za-z0-9_]*=[^[:space:]]*)[[:space:]]+)*(sudo[[:space:]]+)?(rm|mv|cp|tee|truncate|touch|mkdir|rmdir|chmod|chown|ln|install|rsync|patch|dd|unlink|shred)([[:space:]]|$)'
    printf '%s\n' "$cmd" | grep -qE "$W" && refuse "a subagent never writes under reference/ or lz/"
    printf '%s\n' "$cmd" | grep -qE '(^|[^<>])>{1,2}[[:space:]]*"?[^[:space:]"]*(reference|lz)/' && refuse "a subagent never redirects output under reference/ or lz/"
    printf '%s\n' "$cmd" | grep -qE '(^|[;&|(`]|&&)[[:space:]]*(sed|perl|ruby)[[:space:]]+[^;&|]*-[a-zA-Z]*i' && refuse "a subagent never edits in place under reference/ or lz/"
    printf '%s\n' "$cmd" | grep -qE "${G}(add|rm|mv|restore|checkout|apply|am|stash)([[:space:]]|$)" && \
        printf '%s\n' "$cmd" | grep -qE "${G}(add|rm|mv|restore|checkout|apply|am|stash)[^;&|]*(^|[[:space:]\"'=/])(reference|lz)/" && \
        refuse "a subagent never stages or restores files under reference/ or lz/ (the pin bump is the user's)"
    printf '%s\n' "$cmd" | grep -qE '(^|[;&|(`]|&&)[[:space:]]*(cargo[[:space:]]+fmt|rustfmt|just[[:space:]]+fmt)' && refuse "a subagent never formats anything under reference/"
fi
exit 0
