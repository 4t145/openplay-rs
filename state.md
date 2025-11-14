# 在线德州扑克 客户端与服务器状态转换图

下述状态命名与代码中的 GameStage 对齐：Setup → PreFlop → Flop → Turn → River → Showdown → Finished。

## 1) 客户端状态机（Client FSM）

```mermaid
stateDiagram-v2
direction LR
[*] --> Disconnected

Disconnected --> Connecting: open_app / connect()
Connecting --> Authenticating: ws_open
Authenticating --> Lobby: auth.ok
Authenticating --> Disconnected: auth.fail

Lobby --> JoiningTable: join.table(table_id)
JoiningTable --> Seated: join.ok(snapshot)
JoiningTable --> Lobby: join.fail

Seated --> WaitingHand: table.snapshot | hand.end
WaitingHand --> Dealt: srv.cardsDealt(hole2)
Dealt --> Acting: srv.actionRequest(self)
Dealt --> WaitingOthers: srv.actionRequest(other)

Acting --> WaitingOthers: client.actionSent / srv.actionAck
WaitingOthers --> Acting: srv.actionRequest(self_next)
WaitingOthers --> HandResult: srv.showdown | srv.payout
HandResult --> WaitingHand: srv.nextHand | srv.stageChanged(PreFlop)

Seated --> Leaving: leave.table
Leaving --> Lobby: leave.ok
Leaving --> Disconnected: net.closed

state Reconnect {
  [*] --> Reconnecting
  Reconnecting --> Resync: ws_reopen
  Resync --> Seated: srv.snapshot | delta_replay
  Reconnecting --> Disconnected: timeout
}
Seated --> Reconnect: net.lost
```

要点：
- Reconnect 分区用于断线重连与状态回放（snapshot+增量事件）。
- Acting/WaitingOthers 交替直至下注轮完成或进入下一阶段。

---

## 2) 服务器（牌桌）状态机（Table/Game FSM）

```mermaid
stateDiagram-v2
direction LR
[*] --> TableIdle

TableIdle --> Seating: table.open
Seating --> Setup: >=2 players ready

Setup --> PreFlop: postBlinds + shuffle + dealHole
Setup --> TableIdle: <2 players

state PreFlop {
  [*] --> Betting
  Betting --> Betting: player_action(Fold|Call|Raise|AllIn)
  Betting --> Done: roundComplete | allFold
  Done --> [*]
}
PreFlop --> Flop: burn + reveal(3)

state Flop {
  [*] --> Betting
  Betting --> Betting: player_action(...)
  Betting --> Done: roundComplete | allFold
  Done --> [*]
}
Flop --> Turn: burn + reveal(1)

state Turn {
  [*] --> Betting
  Betting --> Betting: player_action(...)
  Betting --> Done: roundComplete | allFold
  Done --> [*]
}
Turn --> River: burn + reveal(1)

state River {
  [*] --> Betting
  Betting --> Betting: player_action(...)
  Betting --> Done: roundComplete | allFold
  Done --> [*]
}

River --> Showdown: roundComplete | multiAllIn
PreFlop --> Showdown: allFold(>=1)  --> note right: 直接结算
Flop --> Showdown: allFold(>=1)
Turn --> Showdown: allFold(>=1)

Showdown --> Payout: evaluateHands + sidePots
Payout --> Cleanup: distribute + reset_round_vars
Cleanup --> Setup: >=2 active -> next_hand
Cleanup --> TableIdle: <2 active
```

要点：
- 每个街道都有 Betting 子状态，完成条件为“所有未全下玩家下注额相等且已行动，或只剩一名未弃牌玩家”。
- 任何街道出现 allFold 直接进入 Showdown/Payout（无需发完公共牌）。
- 支持边池（side pots）与多人 All-In。

---

## 3) 典型行动时序（Sequence：加注一例）

```mermaid
sequenceDiagram
autonumber
participant C2 as Client P2
participant S as Server Table
participant C1 as Client P1
participant C3 as Client P3

Note over S: 状态: PreFlop/Betting, 当前轮到 P2

S-->>C2: actionRequest{turn=P2, minRaise, toCall}
C2->>S: action.raise{amount, seq, ts}
S->>S: validate(turn, minRaise, stack, betFlow)
S-->>C2: actionAck{seq, accepted:true}
S-->>C1: actionEvent{player:P2, Raise, to:amount}
S-->>C3: actionEvent{player:P2, Raise, to:amount}
S-->>C1: potUpdated{main, side[]}
S-->>C2: potUpdated{main, side[]}
S-->>C3: potUpdated{main, side[]}
S-->>C1: turnChanged{next=P3, toCall}
S-->>C2: turnChanged{next=P3, toCall}
S-->>C3: turnChanged{next=P3, toCall}

alt 下注轮完成
  S-->>C1: stageChanged{Flop}
  S-->>C2: stageChanged{Flop}
  S-->>C3: stageChanged{Flop}
  S-->>All: communityCards{add:[c1,c2,c3]}
end
```

补充事件命名建议（简化版）：
- 服务器→客户端：table.snapshot, stage.changed, cards.dealt, community.revealed, action.request, action.event, action.ack, pot.updated, showdown, payout, turn.changed, error, heartbeat
- 客户端→服务器：join.table, leave.table, ready, action.fold/check/call/raise/allin, rebuy, sitout/return, heartbeat

---