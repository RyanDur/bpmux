# phase-1a owner actions (accept / reject)

## appearance

- action buttons only visible in drill-in when:
  - current user = ryan (owner), AND
  - story state = `delivered` OR `ready-for-accept`
- placement: bottom of drill-in body, above any footer
- primary action **Accept** (filled button, brand/green), secondary **Reject** (outlined, amber)

```
┌─ drill-in ──────────────────────────┐
│ … body …                            │
│                                     │
│ [Accept]  [Reject]                  │
└─────────────────────────────────────┘
```

## accept flow

- click Accept → confirmation inline OR direct-fire (owner-gated, low-risk)
- phase-1a: **direct-fire** (no confirmation modal) — owner's intent is deliberate; extra click is friction
- on fire: accept_story MCP call + optimistic UI (buttons disable; "accepted" badge swaps in); on error, revert + error toast
- chat broadcasts the acceptance signal (existing workflow)

## reject flow

- click Reject → inline note-input appears (expandable textarea in place of action bar)
- note required (audit trail); empty note disables Send
- Send/Submit button + Cancel
- on send: reject_story MCP call with note → optimistic UI (buttons disable; "rework needed" badge)
- chat broadcasts the rejection signal with reason

## non-owner view

- when current user is NOT ryan: no Accept/Reject visible
- agents see story status; do NOT see owner-actions
- if story is agent-facing (infra lane per `project_infra_gate_delegation`), pm can team-accept via a separate "team-accept" button shown only to pm when story labeled infra-delegated — phase-1b adds this; phase-1a keeps owner-gate simple

## acceptance-walk visible chrome (per Ryan's direction, visible not action-freeze)

- when ryan enters acceptance-walk mode (clicks Accept/Reject → engaged), the drill-in panel gets a subtle chrome change ("acceptance in progress" banner at top)
- agents see the same banner in their view (via broadcast signal)
- **no action-lock** — agents can still fire MCP actions; discipline holds them via awareness (per `feedback_collaboration_visibility_inviolable`)
- banner dismisses on accept/reject complete or Esc-cancel

## states

- **optimistic**: buttons disable immediately on click; state badge swaps to show pending outcome
- **success**: badge updates to accepted / rejected; drill-in closes OR stays (TBD based on phase-1a UX test)
- **error**: restore buttons; toast error; keep note text preserved in reject flow

## keyboard

- Enter while Accept focused: fires accept (same as click)
- Enter in reject-note textarea: inserts newline (standard textarea)
- Cmd+Enter in reject-note: submits reject (quick-send shortcut)
- Esc during reject flow: cancels back to button view (preserves note text session-locally)

## phase-1b defers

- confirmation modal for Accept (if friction-for-deliberate-intent matters post-usage)
- team-accept button for infra-delegated stories
- bulk accept/reject across multiple stories
- accept-with-optional-note
- reject-reason templates (common reasons prefilled)
