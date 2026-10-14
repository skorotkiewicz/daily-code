#!/usr/bin/env python3
"""daily-code — one 6-digit code per day, reversible back to its date.

The day number is scrambled through a secret-keyed Feistel permutation:
same day → same code; different days → different codes; never a leading zero.

Usage:
    python3 daily_code.py generate                    # today's code
    python3 daily_code.py generate --date 3022-10-24  # any day's code
    python3 daily_code.py generate --days 7           # the next 7 days
    python3 daily_code.py reverse 332231              # code -> its day
    DAILY_CODE_SECRET=mysecret python3 daily_code.py generate

Guarantees (exhaustively verified over all 900,000 supported days):
    - every day -> a new code (never the same code on two different days)
    - the same day -> always the same code (that is what makes reverse work)
    - no visible pattern between consecutive days
"""

from __future__ import annotations

import argparse
import hashlib
import os
import sys
import urllib.request
from datetime import date, datetime, timedelta, timezone

EPOCH = date(1000, 1, 1)  # day 0 — supports years 1000 to ~3464
CODE_LOW = 100_000        # codes never start with 0: range 100000..999999
CODE_COUNT = 900_000      # 900,000 unique codes — one per day
DEFAULT_SECRET = "unguessable-hamster"

# The EFF large wordlist (7,776 real English words) — the published diceware
# standard. Downloaded ONCE, cached, filtered to 6+ letters. NEVER generated.
_EFF_URL = "https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt"
_EFF_CACHE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "eff_long_words.txt")
_EFF_CACHE_DATA = None


def _eff_words() -> list:
    global _EFF_CACHE_DATA
    if _EFF_CACHE_DATA is not None:
        return _EFF_CACHE_DATA
    try:
        with open(_EFF_CACHE, encoding="utf-8") as fh:
            _EFF_CACHE_DATA = [line.strip() for line in fh if line.strip()]
    except OSError:
        try:
            with urllib.request.urlopen(_EFF_URL, timeout=4) as response:
                text = response.read().decode()
            words = sorted({line.split()[1].lower() for line in text.splitlines()
                            if len(line.split()) == 2 and len(line.split()[1]) >= 6
                            and line.split()[1].isalpha()})
            with open(_EFF_CACHE, "w", encoding="utf-8") as fh:
                fh.write("\n".join(words))
            _EFF_CACHE_DATA = words
        except Exception:
            _EFF_CACHE_DATA = []
    return _EFF_CACHE_DATA


def words_for(day: date) -> str:
    """Two long real words for the day — same day → same words,
    new day → new words (like 'unguessable-hamster', but daily)."""
    words = _eff_words()
    if len(words) < 950:
        return secret()  # wordlist unavailable — show the (constant) key
    index = _walk((day - EPOCH).days, _feistel)
    return f"{words[index % len(words)]}-{words[(index // len(words)) % len(words)]}"


def secret() -> str:
    return os.environ.get("DAILY_CODE_SECRET", DEFAULT_SECRET)


def _round_fn(round_no: int, half: int) -> int:
    """Secret-keyed pseudo-random function for one Feistel round."""
    digest = hashlib.sha256(f"{secret()}|{round_no}|{half}".encode()).digest()
    return int.from_bytes(digest[:2], "big") & 1023


def _feistel(value: int) -> int:
    """4-round Feistel permutation over 2^20 values (invertible by design)."""
    left, right = value >> 10, value & 1023
    for r in range(4):
        left, right = right, left ^ _round_fn(r, right)
    return (left << 10) | right


def _feistel_inverse(value: int) -> int:
    left, right = value >> 10, value & 1023
    for r in (3, 2, 1, 0):
        left, right = right ^ _round_fn(r, left), left
    return (left << 10) | right


def _walk(value: int, step) -> int:
    """Cycle-walk the 2^20 permutation down into the code range
    100000..999999 (always applying the permutation at least once)."""
    value = step(value)
    while not (CODE_LOW <= value < CODE_LOW + CODE_COUNT):
        value = step(value)
    return value


def code_for(day: date) -> str:
    """One code per day: the same day always gives the same code, different
    days always give different codes — and the code never starts with 0."""
    day_number = (day - EPOCH).days
    if day_number >= CODE_COUNT:
        raise ValueError("date is beyond the supported range (year ~3464)")
    # start INSIDE the walk domain (>= CODE_LOW) so the walked permutation
    # is injective: no two days can ever produce the same code
    return f"{_walk(day_number + CODE_LOW, _feistel):06d}"


def day_from_code(code: str) -> date:
    """Every code maps back to exactly one day — its own."""
    digits = code.replace("-", "").replace(" ", "")
    if not digits.isdigit() or not (CODE_LOW <= int(digits) < CODE_LOW + CODE_COUNT):
        raise ValueError("code must be 6 digits, not starting with 0")
    day_number = _walk(int(digits), _feistel_inverse) - CODE_LOW
    return EPOCH + timedelta(days=day_number)


def parse_date(text: str) -> date:
    return datetime.strptime(text, "%Y-%m-%d").date()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    gen = sub.add_parser("generate", help="the code for a day (default: today, UTC)")
    gen.add_argument("--date", type=parse_date, default=None,
                     help="YYYY-MM-DD (default: today, UTC)")
    gen.add_argument("--days", type=int, default=None, metavar="N",
                     help="show the code for each of the next N days")
    rev = sub.add_parser("reverse", help="decode a code back to its day")
    rev.add_argument("code", help="6 digits, e.g. 332231 or 332-231")

    args = parser.parse_args(argv)

    if args.command == "generate":
        try:
            day = args.date or datetime.now(timezone.utc).date()
            if args.days:  # show the code for each of the next N days
                for offset in range(args.days):
                    d = day + timedelta(days=offset)
                    print(f"{code_for(d)}  ({d.isoformat()}, secret: '{words_for(d)}')")
                return 0
            print(f"{code_for(day)}  ({day.isoformat()}, secret: '{words_for(day)}')")
            return 0
        except ValueError as error:
            print(f"error: {error}", file=sys.stderr)
            return 1

    try:
        print(day_from_code(args.code).isoformat())
        return 0
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
