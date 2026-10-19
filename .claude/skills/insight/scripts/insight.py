#!/usr/bin/env python3
"""insight.py: a second opinion from external models, through OpenRouter, on one brief.

    python3 -I insight.py BRIEF.md [--models a,b,c] [--out DIR] [--effort high]
                          [--max-tokens N] [--max-input-tokens N] [--timeout S]
                          [--dry-run] [--no-ledger]

The brief is a Markdown file the caller wrote (SKILL.md gives its template): the question,
the measurements so far, the constraints that bind any change, line-numbered excerpts of the
Rust and of the OpenBSD C, and the answer format.  Every model in --models gets the same
brief, in parallel; each raw answer is saved under --out as <stamp>-<model>.md (the text) and
.json (the whole response, reasoning included), a summary table goes to stdout, and one line
per model is appended to the ledger (ledger.tsv next to this skill).  Nothing is applied:
the caller verifies every claim against the code before it becomes work.

Guards: the key is read from OPENROUTER_API_KEY only (never a file in the tree); a brief
over --max-input-tokens (estimated as bytes/3) is refused, so the caller sends ranges, not
files; when the key has a spending limit on OpenRouter, a remaining balance under
--min-remaining USD refuses the call; the worst-case cost (input plus --max-tokens of output
at the models' list prices) is printed before calling.  --dry-run prints the estimate and
stops.  stdlib only.
"""

import argparse
import concurrent.futures
import datetime
import json
import os
import pathlib
import sys
import time
import urllib.error
import urllib.request

API = "https://openrouter.ai/api/v1"
DEFAULT_MODELS = [  # two families, none of them Claude; the user's choice of 2026-10-19 (SKILL.md)
    "google/gemini-4-argon",
    "~openai/gpt-sol-latest",
]  # dropped the same day: qwen/qwen3.8-max-prime (reasoning mandatory, twice the price, two
#  empty answers) and ~deepseek/deepseek-pro-latest (named in Anthropic's distillation report
#  of 2026-02; its answers were the session's own, nothing new)
LEDGER = pathlib.Path(__file__).resolve().parent.parent / "ledger.tsv"
LEDGER_HEADER = "date\tbrief\tmodel\tin_tokens\tout_tokens\tcost_usd\tseconds\tfinish\tfile\n"

SYSTEM = """You are an independent reviewer consulted by the engineer redesigning EmiBSD, an
OpenBSD-derived operating system kernel written in Rust.  The brief gives one question, the
measurements made so far, the constraints that bind any change, line-numbered excerpts of the
Rust, and line-numbered excerpts of the OpenBSD C that is the behavioural specification.

Answer only from the excerpts.  Reply as numbered claims, at most ten, each with:
(1) the claim, one sentence;
(2) evidence: file:line references into the excerpts;
(3) the mechanism: why it costs, or why it is wrong, in two or three sentences;
(4) the C: what the C excerpt does at the same place (its file and line) and whether the Rust
    does more, the same, or less work than it; when the brief gives no C for a claim, say so;
(5) confidence: high, medium or low; and the word "inference" on anything the excerpts do
    not show directly.

Do not write code.  Do not restate the brief.  Do not give general advice.  A claim without
file:line evidence is not wanted; say "no claim" rather than guess.  End with one line: the
claim you would verify first and the measurement that would confirm or refute it."""


def die(msg, code=2):
    print(f"insight: {msg}", file=sys.stderr)
    sys.exit(code)


def api_key():
    k = os.environ.get("OPENROUTER_API_KEY")
    if not k:
        die("OPENROUTER_API_KEY is not set in this environment (the user sets it; it never goes in the tree)")
    return k


def http(method, path, key=None, body=None, timeout=60):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(API + path, data=data, method=method)
    if key:
        req.add_header("Authorization", f"Bearer {key}")
    req.add_header("Content-Type", "application/json")
    req.add_header("X-Title", "EmiBSD insight")
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.load(r)


def key_limits(key):
    d = http("GET", "/auth/key", key=key)["data"]
    return d.get("limit"), d.get("limit_remaining"), d.get("usage")


def prices(models):
    """model -> (usd per input token, usd per output token), from the public model list."""
    out = {}
    try:
        data = http("GET", "/models")["data"]
    except Exception as e:  # noqa: BLE001  (an estimate, never a stop)
        print(f"insight: could not fetch prices ({e!r}); worst-case cost unknown", file=sys.stderr)
        return out
    for m in data:
        if m["id"] in models:
            p = m.get("pricing", {})
            out[m["id"]] = (float(p.get("prompt", 0)), float(p.get("completion", 0)))
    return out


def ask(model, brief, key, max_tokens, effort, timeout):
    body = {
        "model": model,
        "messages": [
            {"role": "system", "content": SYSTEM},
            {"role": "user", "content": brief},
        ],
        "max_tokens": max_tokens,
        "temperature": 0.2,
        "usage": {"include": True},
    }
    body["reasoning"] = {"effort": effort} if effort else {"enabled": False}
    t0 = time.monotonic()
    note = ""
    try:
        try:
            resp = http("POST", "/chat/completions", key=key, body=body, timeout=timeout)
        except urllib.error.HTTPError as e:
            text = e.read()[:600].decode(errors="replace")
            if e.code == 400 and not effort and "easoning is mandatory" in text:
                # Qwen3.8 Max Prime (2026-10-19) cannot run without reasoning: retry at low.
                body["reasoning"] = {"effort": "low"}
                note = "reasoning mandatory for this model: ran at effort low"
                print(f"insight: {model}: {note}", file=sys.stderr)
                resp = http("POST", "/chat/completions", key=key, body=body, timeout=timeout)
            else:
                raise
    except urllib.error.HTTPError as e:
        text = e.read()[:600].decode(errors="replace")
        return {"model": model, "error": f"HTTP {e.code}: {text}", "seconds": time.monotonic() - t0}
    except Exception as e:  # noqa: BLE001
        return {"model": model, "error": repr(e), "seconds": time.monotonic() - t0}
    secs = time.monotonic() - t0
    if "error" in resp:
        return {"model": model, "error": json.dumps(resp["error"])[:600], "seconds": secs, "raw": resp}
    ch = resp["choices"][0]
    usage = resp.get("usage") or {}
    details = usage.get("completion_tokens_details") or {}
    return {
        "model": model,
        "content": ch["message"].get("content") or "",
        "reasoning": ch["message"].get("reasoning"),
        "finish": ch.get("finish_reason"),
        "in": usage.get("prompt_tokens"),
        "out": usage.get("completion_tokens"),
        "reasoning_tokens": details.get("reasoning_tokens"),
        "cost": usage.get("cost"),
        "seconds": secs,
        "note": note,
        "raw": resp,
    }


def slug(model):
    return model.lstrip("~").replace("/", "_").replace(":", "_")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("brief", type=pathlib.Path)
    ap.add_argument("--models", default=",".join(DEFAULT_MODELS), help="comma-separated OpenRouter model ids")
    ap.add_argument("--out", type=pathlib.Path, default=pathlib.Path("target/insight"), help="where the answers are saved")
    ap.add_argument("--effort", default="none", choices=["none", "low", "medium", "high"], help="reasoning effort; none (default) disables it: measured 2026-10-19, reasoning found nothing more and at high it ate the whole cap")
    ap.add_argument("--max-tokens", type=int, default=16000, help="output cap per model, reasoning included where the provider counts it")
    ap.add_argument("--max-input-tokens", type=int, default=40000, help="refuse a brief estimated above this (bytes/3)")
    ap.add_argument("--min-remaining", type=float, default=1.0, help="refuse when the key's remaining limit (USD) is below this")
    ap.add_argument("--timeout", type=int, default=900, help="seconds per model")
    ap.add_argument("--dry-run", action="store_true", help="estimate and stop")
    ap.add_argument("--no-ledger", action="store_true", help="do not append to the ledger")
    a = ap.parse_args()

    brief_text = a.brief.read_text(encoding="utf-8")
    est_in = (len(brief_text.encode()) + len(SYSTEM)) // 3  # code: ~3 bytes/token (measured 2026-10-19)
    models = [m.strip() for m in a.models.split(",") if m.strip()]
    if est_in > a.max_input_tokens:
        die(f"brief estimated at {est_in} tokens, above --max-input-tokens {a.max_input_tokens}: send ranges, not files")

    price = prices(set(models))
    worst = 0.0
    print(f"brief: {a.brief}  (~{est_in} input tokens)")
    for m in models:
        if m in price:
            pin, pout = price[m]
            w = est_in * pin + a.max_tokens * pout
            worst += w
            print(f"  {m}: worst case ${w:.2f} (${pin*1e6:.2f}/M in, ${pout*1e6:.2f}/M out)")
        else:
            print(f"  {m}: not in the model list (unknown id?)")
    print(f"worst-case total: ${worst:.2f}")
    if a.dry_run:
        return

    key = api_key()
    limit, remaining, used = key_limits(key)
    if limit is not None:
        print(f"key: limit ${limit}, used ${used}, remaining ${remaining}")
        if remaining is not None and remaining < a.min_remaining:
            die(f"remaining ${remaining} is below --min-remaining ${a.min_remaining}")
    else:
        print(f"key: no spending limit set on OpenRouter (used ${used} so far); set one on the dashboard")

    a.out.mkdir(parents=True, exist_ok=True)
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    effort = None if a.effort == "none" else a.effort
    with concurrent.futures.ThreadPoolExecutor(max_workers=len(models)) as ex:
        results = list(ex.map(lambda m: ask(m, brief_text, key, a.max_tokens, effort, a.timeout), models))

    ok = 0
    rows = []
    for r in results:
        base = f"{stamp}-{slug(r['model'])}"
        md_path, json_path = a.out / (base + ".md"), a.out / (base + ".json")
        if "raw" in r:
            json_path.write_text(json.dumps(r["raw"], indent=1), encoding="utf-8")
        if "error" in r:
            rows.append((r["model"], "-", "-", "-", f"{r['seconds']:.0f}", "ERROR", r["error"][:80]))
            continue
        if not r["content"].strip():
            rows.append((r["model"], str(r["in"]), str(r["out"]), "-", f"{r['seconds']:.0f}", str(r["finish"]), "EMPTY: no content (reasoning ate the cap?)"))
            continue
        ok += 1
        head = (
            f"# insight: {r['model']}\n\n"
            f"brief: {a.brief}  \nwhen: {stamp}  \nfinish: {r['finish']}  \n"
            f"tokens: in {r['in']}, out {r['out']} (reasoning {r['reasoning_tokens']})  \n"
            f"cost: ${r['cost']}  \nseconds: {r['seconds']:.0f}  \nnote: {r['note'] or '-'}\n\n"
            "Raw opinion.  Not verified.  Nothing here is applied before each claim is checked "
            "against the code and the C (SKILL.md, the verdict table).\n\n---\n\n"
        )
        md_path.write_text(head + r["content"], encoding="utf-8")
        cost = f"{r['cost']:.4f}" if isinstance(r["cost"], (int, float)) else "-"
        rows.append((r["model"], str(r["in"]), str(r["out"]), cost, f"{r['seconds']:.0f}", str(r["finish"]), str(md_path)))
        if not a.no_ledger:
            new = not LEDGER.exists()
            with LEDGER.open("a", encoding="utf-8") as f:
                if new:
                    f.write(LEDGER_HEADER)
                f.write("\t".join([stamp[:8], "/".join(a.brief.resolve().parts[-2:]), r["model"], str(r["in"]), str(r["out"]), cost, f"{r['seconds']:.0f}", str(r["finish"]), md_path.name]) + "\n")

    print()
    print(f"{'model':34} {'in':>7} {'out':>7} {'cost':>8} {'s':>5} {'finish':8} file")
    for row in rows:
        print(f"{row[0]:34} {row[1]:>7} {row[2]:>7} {row[3]:>8} {row[4]:>5} {row[5]:8} {row[6]}")
    if ok == 0:
        die("no model answered", 1)


if __name__ == "__main__":
    main()
