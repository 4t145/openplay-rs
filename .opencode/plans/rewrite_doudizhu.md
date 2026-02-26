# Plan: Rewrite Doudizhu Game Logic

I will rewrite `games/doudizhu` to align with the new `Game` trait and infrastructure changes.

## 1. Modify `games/doudizhu/src/lib.rs`

### Update Structs
- Add `version: u32` to `DouDizhuGame` struct to support optimistic locking.
- Add `timer_id: Option<Id>` to track active timer (if needed for cancellation, though generic logic might handle it).

### Implement `Game` Trait
- Update `handle_action` signature to: `fn handle_action(&mut self, ctx: &RoomContext, event: SequencedGameUpdate) -> GameUpdate`.
- Handle `GameEvent::GameStart`:
    - Perform initialization (shuffle, deal).
    - Set `self.version = 1`.
    - Generate `GameCommand::CreateTimer` for the first player (e.g., 30s for bidding).
    - Return `GameUpdate` with the new state snapshot.
- Handle `GameEvent::Action(action)`:
    - Validate `event.seq == self.version`. If mismatch, ignore/reject.
    - Validate `action.source` (must be current player).
    - Deserialize action payload.
    - Process game logic (Bid/Play/Pass).
    - If valid:
        - Increment `self.version`.
        - Cancel previous timer (`GameCommand::CancelTimer`).
        - Start new timer for next player (`GameCommand::CreateTimer`).
        - Return `GameUpdate` with events and snapshot.
- Handle `GameEvent::TimerExpired(expired)`:
    - Verify timer ID matches current turn timer.
    - Execute default action (Auto-Pass or Auto-Play lowest).
    - Increment `self.version`.
    - Advance turn and set new timer.

### View Handling
- `GameUpdate` requires `views: HashMap<RoomView, GameViewUpdate>`.
- Since Doudizhu has hidden information (hand cards), I must generate different views:
    - `RoomView::Position(pos)`: Sees own hand + public info.
    - `RoomView::Neutral` (Observers): Sees only public info (or everyone's cards if spectator mode allowed, usually just public: played cards, hand counts).
- **Crucial**: Filter `self.players[i].hand` when generating snapshots for other players.

## 2. Update `games/doudizhu/tests/game_logic.rs`

- Update test setup to use `SequencedGameUpdate`.
- Replace direct `game.start()` calls with `handle_action(..., GameEvent::GameStart)`.
- Update `make_message` helper to return `SequencedGameUpdate` with correct `seq`.
- Verify `seq` increments in tests.
- Add test case for Timer Expiration (mocking `TimerExpired` event).

## 3. Verify `games/doudizhu/src/bot.rs`

- Check if `SimpleBotLogic::decide` needs updates. It seems to operate on `DouDizhuGame` struct directly. If `DouDizhuGame` structure changes (added `version`), it might need minor updates but logic should hold.
- The return type `Option<Action>` is fine, caller handles wrapping.

## 4. Dependencies

- Ensure `openplay_basic` and `openplay_poker` versions are compatible (they are in workspace).

## Execution Steps

1.  Read `games/doudizhu/src/lib.rs` again to ensure I have all imports right.
2.  Rewrite `games/doudizhu/src/lib.rs`.
3.  Rewrite `games/doudizhu/tests/game_logic.rs`.
4.  Run tests to verify.
