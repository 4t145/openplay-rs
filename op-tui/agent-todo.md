# Agent TODO (Deferred)

These TUI tasks are intentionally deferred while Bevy adaptation is prioritized.

- Adapt room seat rendering to `RoomState.positions.seats` and show vacant seats.
- Remove hardcoded `1..3` assumptions; drive seat choices from room seat data.
- Update sit/add-bot/kick key flows to align with current seat ids.
- Enforce no direct `Player -> Player` seat switching in TUI interaction flow.
- Re-check ready/start hints and player count text with fixed/flexible seat modes.
- Sync TUI room sidebar behavior with Bevy tweaks (collapsed mode shows latest message only).
- Add owner-only AddBot affordance on vacant seat rows (while preserving sit flow rules).
- Remove rounded corners in TUI style layer for consistency with current test UI direction.
