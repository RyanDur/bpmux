# Board view: story detail — drill into a story

*User selects a story from the board and presses Enter.*

```
┌ #191 topic channels ────────────────────────────────────────────────────────┐
│                                                                              │
│  State: delivered              Reporter: designer                            │
│  Dev: dev                      Filed: 2026-04-12                             │
│                                                                              │
│  Context coherence — topic-scoped channels with delivery-side                │
│  filtering. Messages belong to topics. Participants are opt-in.              │
│  Sidebar shows topics with unread counts.                                    │
│                                                                              │
│  Mockups: docs/design/topic-channels/                                        │
│                                                                              │
│  ──────────────────────────────────────────────────────────────              │
│  ACs:                                                                        │
│  ✓ topic creation with participant list                                      │
│  ✓ delivery-side filtering (non-participants don't receive)                  │
│  ✓ sidebar topic display with active marker                                  │
│  ✓ REST/MCP topic support                                                    │
│    cross-topic unread notification                                           │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
┌ ryan > ──────────────────────────────────────────────────────────────────────┐
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
 #191 detail | Esc=back to board                   ⌥1=topics Ctrl+K=actions
```

## Visual notes

- **Header**: story number + title
- **Metadata row**: state, reporter, dev assigned, filed date
- **Description**: first few lines of the story body (enough context to understand)
- **Mockup reference**: link to design files if they exist
- **AC checklist**: ✓ for completed, blank for pending — glanceable progress
- **Fits one screen**: no scrolling for typical stories

## Colors (Dracula palette)

- Story number in header: DRACULA_CYAN
- State label: color matches the board column (delivered = DRACULA_YELLOW)
- ✓ completed ACs: DRACULA_GREEN
- Pending ACs: DRACULA_COMMENT (dim)
- Description text: DRACULA_FG
- Metadata labels: DRACULA_COMMENT, values: DRACULA_FG

## Interaction

- Esc returns to the board view (not chat — preserves navigation stack)
- Arrow keys scroll if description is longer than viewport
- Accept/reject actions could live here in future (owner-only)
