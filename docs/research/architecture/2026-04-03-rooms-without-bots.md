# How Rooms Got AI Without Building AI

Research into how chat platforms enabled bot/AI participation. The core question: what did platforms build vs. what did ecosystems build, and what does that mean for bpmux?

## 1. IRC: The Accidental Bot Platform

**What IRC provided:** A text-based protocol. Nothing else. No bot API, no bot framework, no bot concept. IRC (RFC 1459) defined how clients connect, join channels, and send messages. A bot is just a client that happens to be automated.

**What the community built:** Everything.

- First bots appeared ~1990-1993 (2-5 years after IRC launched in 1988). Jyrki Alakuijala's Puppe, Greg Lindahl's Game Manager (Hunt the Wumpus), Bill Wisner's Bartender.
- Eggdrop (December 1993) became the first major bot framework. Written to manage a single channel (#gayteen on EFnet). Still in active development 30+ years later.
- Eggdrop introduced Tcl scripting, making bots extensible without recompiling.
- By 1998, most channels had bots. They were the rule, not the exception.

**The pattern:** IRC's contribution was a simple, open protocol where any TCP client could participate. No authentication barriers, no app store, no approval process. The protocol made no distinction between humans and programs. That was the entire enablement story.

**Time to useful bots:** ~2-5 years. But IRC had millions of users providing demand. The ecosystem bootstrapped itself through sheer volume.

**Key insight:** IRC bots emerged because the protocol was so simple that connecting a script was trivial. The barrier was near-zero. But it took years and a massive user base to get there.

## 2. Slack: Deliberate Platform Play

**What Slack provided:**

- Slackbot (built-in, minimal, ships day one) -- handles basic reminders, responds to custom triggers
- Real Time Messaging API (WebSocket-based, deprecated in favor of Events API)
- Events API (HTTP webhook-based, simpler to deploy)
- Incoming Webhooks (send messages into channels via HTTP POST)
- Outgoing Webhooks (trigger external services from channel messages, now legacy)
- OAuth-based authentication for bot users
- Bot Users as first-class entities with their own presence, names, avatars
- Slash commands (custom /commands that hit external endpoints)
- Interactive messages (buttons, menus, modals)
- App Directory (launched December 2015, ~150 integrations at launch)
- $80M venture fund for bot/app developers

**What the ecosystem built:** The actual bots. Thousands of them. GitHub integration, Jira integration, standup bots, poll bots, analytics bots, AI chatbots.

**Timeline:**

- Slack launched August 2013
- API and integrations available from early on (webhooks, basic integrations)
- Classic Apps with OAuth: November 2015 (~2 years post-launch)
- App Directory: December 2015, 150+ integrations
- By 2016, the bot ecosystem was Slack's key differentiator vs. competitors

**Could Slack have succeeded without Slackbot?** Probably yes. Slackbot was a demo, not the value. The value was: (a) making bot creation trivially easy via webhooks, and (b) making bot discovery easy via the App Directory. Slackbot showed what was possible, but the ecosystem delivered the actual utility.

**What made it easy:** Incoming webhooks. You could send a message to Slack with a single curl command. No SDK required, no WebSocket management, no state. Just POST JSON to a URL. This was the on-ramp that hooked developers.

## 3. Discord: Developer-First Ecosystem

**What Discord provided:**

- Official Bot API (April 2016, ~1 year after Discord's May 2015 launch)
- Gateway API (WebSocket for real-time events)
- REST API for sending messages, managing servers
- OAuth2 for bot authentication
- Rich embeds (structured messages with images, fields, colors)
- Slash commands (structured command interface)
- Permissions system (granular bot permissions per server/channel)
- Bot verification program (for bots in 75+ servers)
- Developer portal for managing bot applications

**What the ecosystem built:**

- discord.js, discord.py, and dozens of community libraries in every language
- Top.gg (bot directory with ratings, reviews, discovery)
- MEE6, Carl-bot, Dyno -- moderation bots serving millions of servers
- Music bots, RPG bots, economy bots, AI bots
- An entire cottage industry of bot hosting services

**Timeline:**

- Discord launched May 2015
- Unofficial API explored by community: December 2015 (~7 months)
- Official Bot API + first bot (Airhorn Solutions): April 1, 2016 (~11 months)
- Airhorn Solutions sent 500K messages in 8K servers in its first week
- GameBridge SDK: December 2016
- Rich Presence SDK: December 2017

**What made it easy:** A Discord bot in Python or JavaScript is ~10-30 lines of code. Import library, create client, register event handler for messages, login with token. The community libraries (discord.js, discord.py) abstracted away all WebSocket management. You just write `on_message` handlers.

**Key insight:** Discord shipped the API fast (under a year) but the community libraries are what made bot development accessible. Discord provided the protocol and permissions; the community provided the DX.

## 4. Matrix: Federated Bot Infrastructure

**What Matrix provides:**

- Client-Server API (bots are just clients, like IRC)
- Application Service API (for bridges and multi-user bots)
- Bridge infrastructure (connect Matrix to Slack, Discord, IRC, Telegram, etc.)
- Room-level permissions and power levels
- End-to-end encryption (bots can participate in encrypted rooms)

**What the ecosystem built:**

- Maubot: plugin-based bot framework. Install plugins without writing code.
- Matrix-LLM-Bot, maubot-llm: AI/LLM integration plugins connecting to OpenAI, Anthropic, local models via OpenAI-compatible APIs
- Hundreds of bridges (Slack, Discord, IRC, Telegram, WhatsApp, Signal)
- t2bot.io: hosted bridge and bot service

**AI integration specifics:** maubot-llm connects to any OpenAI-compatible endpoint (including Anthropic). Per-room configuration of models, system prompts, and backends. Commands like `!llm info`, `!llm model`, `!llm system` for in-room configuration. AI moderation plugin uses LLMs for content moderation.

**Key insight:** Matrix's bridge architecture is unique. A bot doesn't just live in Matrix -- it can bridge entire ecosystems. A single Matrix room can have participants from Slack, Discord, IRC, and AI bots simultaneously. The Application Service API is the enabler.

## 5. Zulip: Structured Bot Framework

**What Zulip provides:**

- Interactive bot framework with a defined handler interface
- Two required methods: `usage(self)` and `handle_message(self, message, bot_handler)`
- `bot_handler` for sending messages, accessing storage
- Persistent storage via `bot_handler.storage` (backed by Zulip's database)
- Zulip Botserver (Flask-based, production-ready bot hosting)
- `call_on_each_message` API for quick development
- Testing framework for bots

**What the ecosystem built:** Plugins for various integrations, AI bot wrappers connecting to LLM APIs via FastAPI.

**The pattern:** Zulip is the most opinionated. It defines an interface contract (`handle_message`), provides storage, provides a hosting solution. The tradeoff: easier to build simple bots, harder to build novel ones that don't fit the handler pattern.

**Key insight:** Zulip's `bot_handler.storage` is notable. Most platforms force bots to manage their own state. Zulip provides persistent key-value storage in the platform's database. This removes a significant barrier for bot developers.

## 6. The Common Pattern

| Platform | Protocol | Event System | Bot Identity | Permissions | Storage | Discovery | Framework |
|----------|----------|-------------|-------------|-------------|---------|-----------|-----------|
| IRC      | Yes      | No (raw)    | No (just clients) | No     | No      | No        | No        |
| Slack    | Yes      | Yes (Events API) | Yes (bot users) | Yes (OAuth scopes) | No | Yes (App Directory) | No (SDKs only) |
| Discord  | Yes      | Yes (Gateway) | Yes (bot users) | Yes (granular) | No | Community (top.gg) | No (community libs) |
| Matrix   | Yes      | Yes (CS API) | Yes (appservice) | Yes (power levels) | No | No | Community (maubot) |
| Zulip    | Yes      | Yes (handler) | Yes (bot users) | Limited | Yes (built-in) | No | Yes (built-in) |

**What every platform provides (minimum viable bot platform):**

1. **A protocol that bots can speak.** This is non-negotiable. Whether it's raw TCP (IRC), WebSocket (Discord/Slack RTM), HTTP (Slack Events), or a client API (Matrix), bots need a way in.
2. **Message events.** The bot needs to know when someone speaks. This can be as simple as "new text in channel" or as structured as typed event objects.
3. **A way to send messages back.** The bot needs to respond. HTTP POST, WebSocket send, or client API call.

**What successful platforms add (but aren't strictly required):**

4. **Bot identity.** A distinct user type for bots (name, avatar, presence). IRC skipped this; everyone else provides it.
5. **Permissions/scoping.** What can the bot read? Write? Which channels? OAuth scopes (Slack), permission intents (Discord), power levels (Matrix).
6. **Structured events.** Beyond raw text: reactions, thread replies, file uploads, interactive components.

**What no platform provides (the ecosystem always builds):**

- The actual bot logic
- Bot frameworks and libraries that wrap the protocol
- Bot hosting infrastructure
- Bot discovery directories (except Slack's App Directory)
- AI/LLM integration

## 7. Time to Ecosystem

| Platform | Launch | First Bots | Bot API | Mature Ecosystem | Time to First Bot | Time to Ecosystem |
|----------|--------|-----------|---------|-----------------|-------------------|-------------------|
| IRC | 1988 | ~1990 | N/A (protocol is the API) | ~1998 | ~2 years | ~10 years |
| Slack | Aug 2013 | ~2014 (webhooks) | Nov 2015 | ~2016 | ~1 year | ~3 years |
| Discord | May 2015 | Dec 2015 (unofficial) | Apr 2016 | ~2017-2018 | ~7 months | ~2-3 years |
| Matrix | 2014 | ~2016 | 2016 (appservice) | ~2019-2020 | ~2 years | ~5-6 years |
| Zulip | 2012 (internal), 2015 (open source) | ~2016 | Built-in | Limited | ~1 year | Still niche |

**The trend:** Each generation gets faster. IRC took a decade. Slack took 3 years. Discord took 2. But every one of them had a large user base driving demand. The ecosystem formed because developers wanted bots for their own communities.

**Is this timeline viable for bpmux?** No. bpmux has one user. Even Discord's fast timeline (7 months to first community bot) assumed thousands of servers and millions of users creating demand. There is no demand signal for a third-party bpmux bot ecosystem.

## 8. The Cold Start Problem

bpmux faces a fundamentally different situation than any of these platforms:

**What every successful bot ecosystem had that bpmux doesn't:**

- A large user base creating demand for bots
- Developers who were already users and wanted to scratch their own itch
- Network effects (bots attracted users, users attracted bot developers)
- Time -- even the fastest took months with millions of users

**The cold start literature says:** Single-user utility is the most common way to bootstrap. Give one person a tool that's useful alone. As a byproduct, they create content/context that becomes the seed of a network. Pinterest is the canonical example -- useful as a personal bookmarking tool, which happened to build a social graph.

**Applied to bpmux:**

1. **"The ecosystem will build bots" is not viable.** There is no ecosystem. There won't be one for a long time. Waiting for it is waiting for Godot.

2. **A built-in bot is not the answer either.** Slackbot didn't make Slack successful. The API made Slack successful. Slackbot was a demo. Building a bpmux-specific AI bot couples the product to a specific AI implementation and misses the point.

3. **What bpmux actually needs is what every platform needed: a protocol that bots can speak.**

The minimum viable bot platform for bpmux:

- **Message protocol:** A way for a non-TUI process to send and receive messages in a session. This could be as simple as writing to and reading from the JSONL log.
- **Bot identity:** A way to distinguish bot messages from human messages in the log (a `participant_type` field or similar).
- **That's it.** No framework, no SDK, no hosting, no discovery. Just: can a process participate in the conversation?

If the JSONL session log is the protocol (and it already exists), then the "bot API" is: tail the log, parse JSON lines, append new JSON lines. A shell script could be a bot. A Claude Code session could be a bot. The protocol IS the product.

## Synthesis: What This Means for bpmux

**The historical pattern is clear:**

1. Platform provides protocol + message events + send capability
2. Community builds everything else
3. This takes months to years and requires a large user base

**bpmux's situation is different:**

- No user base to create ecosystem demand
- The "bot" bpmux needs most (AI pair programmer) already exists as external tools (Claude Code, Cursor, Copilot)
- The value proposition isn't "we have great bots" -- it's "your existing AI tools can participate transparently"

**The insight from this research:** The platforms that succeeded fastest (Discord, Slack) didn't succeed because they built great bots. They succeeded because they made their protocol so easy to speak that building a bot was trivial. Discord: 10 lines of code. Slack: one curl command to an incoming webhook.

**For bpmux, the equivalent is:** Can an AI tool join a session by reading/writing JSONL? If yes, the "bot API" already exists. The work isn't building a bot framework -- it's making the existing message log accessible to external processes and ensuring the protocol is documented well enough that any tool can participate.

**The cold start solution isn't to build a bot. It's to make the room speakable.**
