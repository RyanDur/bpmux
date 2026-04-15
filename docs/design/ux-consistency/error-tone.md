# UX spec: error message tone

*"Good looks like" target for consistent error messages across the TUI.*

## Rule

All user-facing error messages follow one pattern:

```
Sentence case. No prefix. Period at end.
```

## Examples — current → target

```
CURRENT                                          TARGET
─────────────────────────────────────────────────────────────────────
/share: no recent message in the current view    No recent message in the current view.
  to share.

/share: cannot share a private message...        Cannot share a private message.

tokens: unable to read log: {e}                  Unable to read token log.

identity: {e}                                    Unable to load identity.

no channel matches "{arg}"...                    No channel matches "{arg}".

Roadmap unavailable: {e}                         Roadmap unavailable.

Board unavailable: {e}\n Ensure...               Board unavailable.

P2P error: {e}                                   Connection failed.

focus: {joined}                                  Focus: {joined}.

focus: off (all roles visible)                   Focus cleared.

identity ({role}): no traits defined yet.        No traits defined yet.
```

## Also applies to: usage/help text

All `/command` usage text should follow the same multi-line format
(see share-usage.md). Current offenders:

```
CURRENT                                          TARGET
─────────────────────────────────────────────────────────────────────
/to usage: single-line blob                      Multi-line, one form per line
/share usage: 144-char single line               Multi-line (see share-usage.md)
/join, /accept: "Usage: /join ..."               Drop "Usage:" prefix
```

## The three rules

1. **Sentence case** — first word capitalized, rest lowercase (except proper nouns)
2. **No command prefix** — don't echo the command name back (`/share:`, `tokens:`, `identity:`)
3. **No raw error strings** — never surface `{e}` to the user. Write a human sentence. Log `{e}` to the session log for debugging

## Why no prefix?

The user just typed the command. They know what they ran. Echoing it back wastes 10+ characters of an 80-column line and reads like a stack trace, not a message.

## Why no raw errors?

`Roadmap unavailable: reqwest::Error { kind: Connect... }` means nothing to a human user. `Roadmap unavailable.` is enough. If dev needs the raw error, it goes to the session log.

## Tone

Informational, not apologetic. No "Sorry, ...", no "Oops!". State what happened.

```
GOOD                              BAD
──────────────────────────────────────────────────
No results for "auth".            Sorry, no results found!
Focus cleared.                    Oops, focus was cleared.
Topic "design" already exists.    Error: topic already exists!!
```
