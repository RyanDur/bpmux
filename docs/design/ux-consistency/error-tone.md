# UX spec: system message tone

*"Good looks like" target for consistent system messages across the TUI.*

## Three message types, three rules

### Errors (something went wrong)

```
Sentence case. No prefix. No raw errors. Period at end.
```

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
```

Rules for errors:
1. **Sentence case** — first word capitalized, rest lowercase
2. **No command prefix** — don't echo the command name back
3. **No raw error strings** — never surface `{e}` to the user. Log it
   for debugging
4. **Period at end**

### Confirmations (command succeeded)

```
Sentence case. Command context is fine. Period at end.
```

```
EXAMPLES (these are correct as-is)
─────────────────────────────────────────────────────────────────────
Focus: dev, pm.                    (tells user what state they're in)
Focus cleared.                     (tells user focus is off)
Topic "design" created.            (confirms the action)
```

Confirmations can echo the command context because the user needs to
know what state they've entered. "Focus: dev, pm." is more useful than
just "Done."

### Data display (showing information)

```
Header format is fine. Structured output follows the header.
```

```
EXAMPLES (these are correct as-is)
─────────────────────────────────────────────────────────────────────
identity (dev):                    (header for trait listing)
  clarity: ...                     (structured content follows)
```

Data display uses headers to label the output being shown. The prefix
is a label, not a command echo.

## Usage/help text

All `/command` usage text should use multi-line format, one form per
line (see share-usage.md). Drop the "Usage:" prefix.

```
CURRENT                                          TARGET
─────────────────────────────────────────────────────────────────────
Usage: /to <recipients> <message>...             /to dev "hello"    send to dev
                                                 /to dev,pm "hi"    send to multiple

Usage: /join <passphrase> <offer-code>           /join secret abc    join with code

Usage: /accept <answer-code>                     /accept xyz         accept connection
```

## Tone

Informational, not apologetic. No "Sorry, ...", no "Oops!". State what
happened.

```
GOOD                              BAD
──────────────────────────────────────────────────
No results for "auth".            Sorry, no results found!
Focus: dev.                       Oops, focus was set.
Topic "design" already exists.    Error: topic already exists!!
```

## Why no raw errors?

`Roadmap unavailable: reqwest::Error { kind: Connect... }` means
nothing to a human user. `Roadmap unavailable.` is enough. If dev needs
the raw error, it goes to the session log.
