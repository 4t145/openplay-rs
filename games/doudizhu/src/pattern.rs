use openplay_poker::{Card, Rank};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

/// 斗地主中的牌面大小
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DouDizhuRank {
    Three = 3,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
    Two,
    BlackJoker,
    RedJoker,
}

impl PartialOrd for DouDizhuRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DouDizhuRank {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

impl From<&Card> for DouDizhuRank {
    fn from(card: &Card) -> Self {
        match card {
            Card::NaturalCard(c) => match c.rank {
                Rank::Three => DouDizhuRank::Three,
                Rank::Four => DouDizhuRank::Four,
                Rank::Five => DouDizhuRank::Five,
                Rank::Six => DouDizhuRank::Six,
                Rank::Seven => DouDizhuRank::Seven,
                Rank::Eight => DouDizhuRank::Eight,
                Rank::Nine => DouDizhuRank::Nine,
                Rank::Ten => DouDizhuRank::Ten,
                Rank::Jack => DouDizhuRank::Jack,
                Rank::Queen => DouDizhuRank::Queen,
                Rank::King => DouDizhuRank::King,
                Rank::Ace => DouDizhuRank::Ace,
                Rank::Two => DouDizhuRank::Two,
            },
            Card::BlackJoker => DouDizhuRank::BlackJoker,
            Card::RedJoker => DouDizhuRank::RedJoker,
            _ => panic!("Invalid card for Dou Dizhu"),
        }
    }
}

/// 牌型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pattern {
    Single(DouDizhuRank),
    Pair(DouDizhuRank),
    Triple(DouDizhuRank),
    TripleWithSingle(DouDizhuRank, DouDizhuRank), // (Triple, Single)
    TripleWithPair(DouDizhuRank, DouDizhuRank),   // (Triple, Pair)

    // 顺子 (Start rank, length)
    Straight(DouDizhuRank, u8),

    // 连对 (Start rank, length) e.g., 334455 is (3, 3)
    PairSequence(DouDizhuRank, u8),

    // 飞机 (Start rank, length) - 主要是三顺
    Airplane(DouDizhuRank, u8),

    // 飞机带翅膀 (Start rank, length, attached cards)
    AirplaneWithWings(DouDizhuRank, u8),

    Bomb(DouDizhuRank),
    Rocket, // 王炸
}

impl Pattern {
    /// 比较牌型大小
    pub fn beats(&self, other: &Pattern) -> bool {
        // 火箭最大
        if let Pattern::Rocket = self {
            return true;
        }
        if let Pattern::Rocket = other {
            return false;
        }

        // 炸弹比非炸弹大
        if let Pattern::Bomb(_) = self {
            if !matches!(other, Pattern::Bomb(_)) {
                return true;
            }
        }
        if !matches!(self, Pattern::Bomb(_)) {
            if matches!(other, Pattern::Bomb(_)) {
                return false;
            }
        }

        // 同类型比较
        match (self, other) {
            (Pattern::Single(r1), Pattern::Single(r2)) => r1 > r2,
            (Pattern::Pair(r1), Pattern::Pair(r2)) => r1 > r2,
            (Pattern::Triple(r1), Pattern::Triple(r2)) => r1 > r2,
            (Pattern::TripleWithSingle(r1, _), Pattern::TripleWithSingle(r2, _)) => r1 > r2,
            (Pattern::TripleWithPair(r1, _), Pattern::TripleWithPair(r2, _)) => r1 > r2,
            (Pattern::Bomb(r1), Pattern::Bomb(r2)) => r1 > r2,

            (Pattern::Straight(r1, l1), Pattern::Straight(r2, l2)) => l1 == l2 && r1 > r2,
            (Pattern::PairSequence(r1, l1), Pattern::PairSequence(r2, l2)) => l1 == l2 && r1 > r2,
            (Pattern::Airplane(r1, l1), Pattern::Airplane(r2, l2)) => l1 == l2 && r1 > r2,
            (Pattern::AirplaneWithWings(r1, l1), Pattern::AirplaneWithWings(r2, l2)) => {
                l1 == l2 && r1 > r2
            }

            _ => false, // 类型不同且不是炸弹/火箭，无法比较（或者说不能压）
        }
    }
}

/// 分析手牌对应的牌型
pub fn analyze_pattern(cards: &[Card]) -> Option<Pattern> {
    if cards.is_empty() {
        return None;
    }

    let mut ranks: Vec<DouDizhuRank> = cards.iter().map(DouDizhuRank::from).collect();
    ranks.sort();

    let len = ranks.len();

    // 1. 单张
    if len == 1 {
        return Some(Pattern::Single(ranks[0]));
    }

    // 2. 王炸
    if len == 2 && ranks[0] == DouDizhuRank::BlackJoker && ranks[1] == DouDizhuRank::RedJoker {
        return Some(Pattern::Rocket);
    }

    // 3. 对子
    if len == 2 && ranks[0] == ranks[1] {
        return Some(Pattern::Pair(ranks[0]));
    }

    // 统计各个点数的数量
    let mut counts: HashMap<DouDizhuRank, u8> = HashMap::new();
    for &r in &ranks {
        *counts.entry(r).or_insert(0) += 1;
    }

    // 按照数量分组
    let mut count_groups: HashMap<u8, Vec<DouDizhuRank>> = HashMap::new();
    for (&r, &c) in &counts {
        count_groups.entry(c).or_default().push(r);
    }
    // 让每个组内的 rank 有序
    for list in count_groups.values_mut() {
        list.sort();
    }

    // 4. 三张
    if len == 3 && count_groups.get(&3).map_or(false, |v| v.len() == 1) {
        return Some(Pattern::Triple(count_groups[&3][0]));
    }

    // 5. 炸弹
    if len == 4 && count_groups.get(&4).map_or(false, |v| v.len() == 1) {
        return Some(Pattern::Bomb(count_groups[&4][0]));
    }

    // 6. 三带一
    if len == 4 && count_groups.get(&3).map_or(false, |v| v.len() == 1) {
        let triple_rank = count_groups[&3][0];
        let single_rank = count_groups[&1][0];
        return Some(Pattern::TripleWithSingle(triple_rank, single_rank));
    }

    // 7. 三带二 (一对)
    if len == 5
        && count_groups.get(&3).map_or(false, |v| v.len() == 1)
        && count_groups.get(&2).map_or(false, |v| v.len() == 1)
    {
        let triple_rank = count_groups[&3][0];
        let pair_rank = count_groups[&2][0];
        return Some(Pattern::TripleWithPair(triple_rank, pair_rank));
    }

    // 8. 顺子 (5张及以上, 不含2和王, 连续)
    if len >= 5 && count_groups.len() == 1 && count_groups.contains_key(&1) {
        let singles = &count_groups[&1];
        if is_continuous(singles) {
            return Some(Pattern::Straight(singles[0], len as u8));
        }
    }

    // 9. 连对 (3对及以上, 不含2和王, 连续)
    if len >= 6 && len % 2 == 0 && count_groups.len() == 1 && count_groups.contains_key(&2) {
        let pairs = &count_groups[&2];
        if pairs.len() >= 3 && is_continuous(pairs) {
            return Some(Pattern::PairSequence(pairs[0], pairs.len() as u8));
        }
    }

    // 10. 飞机 (不带翅膀) -> 实际上就是连续的三张
    if len >= 6 && len % 3 == 0 && count_groups.len() == 1 && count_groups.contains_key(&3) {
        let triples = &count_groups[&3];
        if triples.len() >= 2 && is_continuous(triples) {
            return Some(Pattern::Airplane(triples[0], triples.len() as u8));
        }
    }

    // 11. 飞机带翅膀
    if count_groups.contains_key(&3) {
        let triples = &count_groups[&3];
        if triples.len() >= 2 && is_continuous(triples) {
            let n = triples.len();
            // 带单牌
            if len == n * 4 {
                return Some(Pattern::AirplaneWithWings(triples[0], n as u8));
            }
            // 带对子
            if len == n * 5 {
                if count_groups.get(&2).map_or(0, |v| v.len()) == n {
                    return Some(Pattern::AirplaneWithWings(triples[0], n as u8));
                }
            }
        }
    }

    None
}

// 辅助函数
fn is_continuous(ranks: &[DouDizhuRank]) -> bool {
    for i in 0..ranks.len() - 1 {
        // 检查是否包含 2 或 Joker
        if ranks[i] >= DouDizhuRank::Two || ranks[i + 1] >= DouDizhuRank::Two {
            return false;
        }
        // 检查是否连续
        if (ranks[i + 1] as u8) != (ranks[i] as u8) + 1 {
            return false;
        }
    }
    true
}
