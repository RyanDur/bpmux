# Remote Notification Options for bpmux

Date: 2026-04-08
Prompted by: Ryan asking "is there a way you can notify me somehow? remotely?"

## Recommendation

**ntfy.sh** — free, self-hostable, one HTTP POST sends a push notification. No account, no API key, aligns with bpmux philosophy (simple transport, no opinions on interaction).

Runner-up: **Pushover** ($5 one-time, polished mobile app) for users who want something turnkey.

## Comparison

| Option | Setup | Cost | Privacy | Account needed |
|--------|-------|------|---------|----------------|
| ntfy.sh | 1/5 | Free (self-hostable) | High (self-host) | No |
| Pushover | 2/5 | $5 one-time | Medium | Yes |
| Telegram Bot | 2/5 | Free | Low (Telegram sees all) | Yes |
| Slack webhook | 2/5 | Free tier | Low (Slack sees all) | Yes |
| Discord webhook | 2/5 | Free | Low (Discord sees all) | Yes |
| Email (SMTP) | 3/5 | Free/varies | Medium | Yes (SMTP server) |
| Twilio SMS | 4/5 | Per-message | Medium | Yes + billing |
| APNs/FCM | 5/5 | Free | Low (Apple/Google) | Yes + dev account |
| PWA push | 4/5 | Free | High | No |

## Why ntfy.sh

- One HTTP POST: `curl -d "Team needs you" ntfy.sh/bpmux-ryan`
- No account, no API key, no signup
- Self-hostable for full privacy control
- Mobile apps for iOS and Android
- Trivial Rust integration: single `reqwest` POST
- Aligns with bpmux philosophy: simple transport, no behavior opinions

## Integration Design

The simplest integration: a bpmux hook or bridge tool that posts to ntfy when an agent explicitly flags the owner. Not automatic for every message — only when the team deliberately needs attention.

```
# Agent calls a bridge tool:
notify_owner("We need your input on the arc architecture direction")

# Bridge posts to ntfy:
POST https://ntfy.sh/bpmux-ryan
Body: "bpmux: We need your input on the arc architecture direction"
```

The user configures their ntfy topic once. The bridge handles the rest.

## Future: General Integration API

This is the first outbound integration. The same pattern (HTTP POST to a configured URL) works for CI notifications, deploy alerts, monitoring. The notification endpoint becomes the first building block of bpmux's integration API.
