#!/usr/bin/env python3
"""test.py — simple test for daily_code.py

Checks the guarantees:
  1. same day   -> same code
  2. every day  -> new code (no two days share a code) — exhaustive
  3. no code starts with 0
  4. reverse    -> exact day
  5. secret     -> different secret, different codes
"""

from datetime import timedelta

import daily_code as dc

failures = 0


def check(name, condition):
    global failures
    print(f"{'PASS' if condition else 'FAIL'}  {name}")
    if not condition:
        failures += 1


# --- 1. same day -> same code ------------------------------------------
c1 = dc.code_for(dc.parse_date("2026-10-14"))
c2 = dc.code_for(dc.parse_date("2026-10-14"))
check("same day gives same code", c1 == c2)

# --- 2+3. exhaustive: every day -> unique code, no leading zero ---------
codes = set()
dups = 0
zeros = 0
for i in range(dc.CODE_COUNT):  # every supported day — exhaustive
    day = dc.EPOCH + timedelta(days=i)
    c = dc.code_for(day)
    if c in codes:
        dups += 1
    if c[0] == "0":
        zeros += 1
    codes.add(c)
check(f"{dc.CODE_COUNT} days -> {len(codes)} unique codes, {dups} duplicates",
      dups == 0 and len(codes) == dc.CODE_COUNT)
check("no code starts with 0", zeros == 0)

# --- 4. reverse -> exact day --------------------------------------------
ok = True
for i in (0, 1, 366, 1017, 100_000, dc.CODE_COUNT - 1):
    day = dc.EPOCH + timedelta(days=i)
    if dc.day_from_code(dc.code_for(day)) != day:
        ok = False
        break
check("reverse(code) == day (6 sample days)", ok)

# --- 5. secret changes the code -----------------------------------------
code_default = dc.code_for(dc.parse_date("2026-10-14"))
original_secret = dc.secret
dc.secret = lambda: "another-secret-42"
code_other = dc.code_for(dc.parse_date("2026-10-14"))
dc.secret = original_secret
check("different secret -> different code", code_default != code_other)
check("reverse still works with original secret",
      dc.day_from_code(code_default) == dc.parse_date("2026-10-14"))

# --- summary -------------------------------------------------------------
print("-" * 46)
print("ALL TESTS PASSED" if failures == 0 else f"{failures} TEST(S) FAILED")
raise SystemExit(0 if failures == 0 else 1)
