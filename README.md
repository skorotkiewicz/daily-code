# daily-code

One 6-digit code per day, reversible back to its date.

- same day → same code
- different days → different codes (never repeated, never a leading zero)
- `reverse` maps a code back to exactly its day

## Usage

```sh
python daily_code.py generate                    # today's code
python daily_code.py generate --date 1991-01-3   # any day's code
python daily_code.py generate --days 7           # the next 7 days
python daily_code.py reverse 668558              # code -> its day
```

Example output:

```
668558  (1991-01-01, secret: 'vanquish-afraid')
```

## Secret

Codes are keyed by a secret. Change it without editing the file:

```sh
DAILY_CODE_SECRET=my-secret python daily_code.py generate
```

Changing the secret changes every code.

## Tests

```sh
python test.py
```
