# Awareness Design: Solving Overload Without Losing Transparency

*Design patterns and philosophies for rendering the bpmux chat without overwhelming the human.*

---

## The Problem

bpmux's transparency principle says everything is visible to everyone. But research on information overload and notification fatigue warns that full visibility can be counterproductive — humans start ignoring everything when everything demands equal attention.

The challenge: how do we keep everything visible (transparency) while making only the right things prominent (attention management)?

## Five Design Philosophies

### 1. Calm Technology (Weiser & Brown 1995, Case 2015)

Technology should move between the center and periphery of attention. Not everything needs to be focal. The periphery is "what we are attuned to without attending to explicitly." By placing things in the periphery, we can be aware of many more things than if everything demanded focal attention.

**Three of Amber Case's eight principles apply directly:**

- **I. Technology should require the smallest possible amount of attention.** AI-to-AI chatter should not demand the human's focal attention by default.
- **III. Technology should make use of the periphery.** AI-to-AI messages belong in the periphery — present but not prominent.
- **VII. The right amount of technology is the minimum needed to solve the problem.** Don't render more than the human needs to maintain awareness.

**Source:** [Amber Case — Principles of Calm Technology](https://calmtech.com/book) | [Weiser & Brown — The Coming Age of Calm Technology (1996)](https://calmtech.com/papers/coming-age-calm-technology)

### 2. Progressive Disclosure (popularized by Nielsen Norman Group)

Show only what's needed now. Defer complexity until it's requested. This reduces cognitive load by presenting the most important information first, with details available on demand.

**Applied to bpmux:** Messages render at different levels of detail by default:
- Human-addressed messages: full content, immediately visible
- AI decisions or questions: summary line visible, full reasoning expandable
- AI-to-AI context: collapsed to sender + topic indicator, expandable on click/keypress

The human sees that activity is happening (awareness) without being forced to process every message (overload). They can expand anything they're curious about (progressive disclosure).

**Source:** [Nielsen Norman Group — Progressive Disclosure](https://www.nngroup.com/articles/progressive-disclosure/)

### 3. Data-Ink Ratio (Tufte 1983)

Maximize the proportion of "ink" (pixels) used to convey information. Remove everything that doesn't carry signal. Tufte calls unnecessary visual elements "chartjunk."

**Applied to bpmux:** Every visual element in the chat should carry information:
- Sender identity and timestamp: data (keep)
- Message content: data (keep, but at appropriate salience level)
- Decorative borders between messages: chartjunk (minimize)
- Repeated context the human already knows: chartjunk (collapse after first view)

This principle also applies to the AI's view: the bounded read window and token budget are data-ink optimization for AI context. Don't fill the context window with low-value messages.

**Source:** [Edward Tufte — The Visual Display of Quantitative Information (1983)](https://www.edwardtufte.com/book/the-visual-display-of-quantitative-information/)

### 4. Information Scent (Pirolli & Card, PARC)

Users forage for information like animals forage for food — they follow "information scent" trails of relevance. Strong scent (clear cues that valuable information is nearby) leads to engagement. Weak scent leads to ignoring or leaving.

**Applied to bpmux:** Messages should vary in information scent based on their relevance to the human:
- **Strong scent:** A message addressed to the human, a decision that needs approval, a conflict between AIs — rendered prominently, maybe with a highlight or badge
- **Medium scent:** An AI announcing what it's working on, a completed action — rendered normally
- **Weak scent:** AI-to-AI technical detail, routine status updates — rendered dimly, collapsed

The human's eye naturally follows the strong scent to the messages that matter, skipping the low-scent background.

**Source:** [Pirolli & Card — Information Foraging Theory (NN/g summary)](https://www.nngroup.com/articles/information-foraging/)

### 5. Tiered Notification Severity

Not all messages deserve the same visual treatment. Research on notification fatigue (Smashing Magazine, 2025) recommends three tiers:
- **High attention:** Requires immediate action (errors, approvals, conflicts)
- **Medium attention:** Worth knowing but not urgent (status updates, completions)
- **Low attention:** Ambient context (AI-to-AI reasoning, routine chatter)

Overusing high-attention rendering creates alert fatigue — users start dismissing everything. The key is reserving high-salience for genuinely important moments.

**Source:** [Smashing Magazine — Design Guidelines for Better Notifications UX (2025)](https://www.smashingmagazine.com/2025/07/design-guidelines-better-notifications-ux/)

---

## Synthesis: A Rendering Model for bpmux

These five philosophies converge on the same design:

### Message Tiers

| Tier | When | Rendering | Example |
|---|---|---|---|
| **Focal** | Message needs human action | Full content, highlighted, persistent until acknowledged | "I see two approaches — which aligns with the client's priorities?" |
| **Active** | Message is informational, human-relevant | Full content, normal rendering | "I'm refactoring the auth module to use the new session type" |
| **Peripheral** | AI-to-AI context, routine status | Collapsed to one line (sender + topic), expandable | "ryan:claude → alice:claude: discussing token refresh strategy" |
| **Ambient** | Heartbeats, acknowledgments, minor status | Minimal indicator (dot, count badge), or hidden until scrolled to | "alice:claude: acknowledged" |

### How It Works

1. **bpmux assigns a tier to each message** based on sender/recipient and content patterns:
   - Human-addressed messages → Focal or Active
   - AI addressing a specific question → Focal
   - AI announcing work → Active
   - AI-to-AI reasoning → Peripheral
   - Acknowledgments, status pings → Ambient

2. **The TUI/Web UI renders each tier differently:**
   - Focal: full brightness, highlighted border, maybe a badge
   - Active: normal brightness, standard rendering
   - Peripheral: dimmed text, collapsed to summary, expandable on click/keypress
   - Ambient: hidden by default, visible in a secondary panel or on scroll

3. **The human can override tiers:**
   - Expand any peripheral message to full view
   - Pin a message to focal (keep it visible)
   - Mute a participant (all their messages → ambient)
   - `/focus` command to hide everything below Active tier

4. **All messages are still in the log.** Transparency is preserved — nothing is deleted or blocked. The tiers control rendering, not storage. The AI's `read_messages` still returns everything within its window, unaffected by visual tiers.

### Design Principles (bpmux-specific)

1. **Transparency is about availability, not salience.** Everything is available. Not everything is prominent.
2. **The human controls the zoom level.** Default tiers are sensible, but the human can adjust.
3. **Reserve focal rendering for genuine decisions.** Overusing it creates alert fatigue.
4. **The AI doesn't know about tiers.** Tiers are a rendering concern, not a message model concern. The `from` field and content determine the tier. The AI sends messages; the TUI/Web UI decides how to render them.
5. **Err toward lower salience.** A message rendered too quietly can be expanded. A message rendered too loudly trains the human to ignore notifications.

---

## Sources

- [Weiser & Brown — The Coming Age of Calm Technology (1996)](https://calmtech.com/papers/coming-age-calm-technology)
- [Amber Case — Calm Technology: Principles and Patterns (2015)](https://calmtech.com/book)
- [Calm Tech Institute — Principles](https://www.calmtech.institute/calm-tech-principles)
- [Nielsen Norman Group — Progressive Disclosure](https://www.nngroup.com/articles/progressive-disclosure/)
- [Edward Tufte — The Visual Display of Quantitative Information (1983)](https://www.edwardtufte.com/book/the-visual-display-of-quantitative-information/)
- [Pirolli & Card — Information Foraging Theory](https://www.nngroup.com/articles/information-foraging/)
- [Smashing Magazine — Notifications UX Guidelines (2025)](https://www.smashingmagazine.com/2025/07/design-guidelines-better-notifications-ux/)
- [Frontiers in Psychology — Dealing with Information Overload (2023)](https://pmc.ncbi.nlm.nih.gov/articles/PMC10322198/)
