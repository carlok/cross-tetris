use crate::piece::PieceKind;
use crate::rng::Rng;
use std::collections::VecDeque;

/// A standard 7-bag randomizer: each bag is a shuffled permutation of all 7
/// piece kinds, refilled whenever it runs out.
#[derive(Clone, PartialEq, Debug)]
pub struct SevenBag {
    rng: Rng,
    queue: VecDeque<PieceKind>,
}

impl SevenBag {
    pub fn new(seed: u64) -> Self {
        let mut bag = SevenBag {
            rng: Rng::new(seed),
            queue: VecDeque::new(),
        };
        bag.refill();
        bag
    }

    fn refill(&mut self) {
        let mut pieces = PieceKind::ALL;
        // Fisher-Yates shuffle.
        for i in (1..pieces.len()).rev() {
            let j = self.rng.next_below((i + 1) as u32) as usize;
            pieces.swap(i, j);
        }
        self.queue.extend(pieces);
    }

    pub fn next(&mut self) -> PieceKind {
        if self.queue.len() <= 7 {
            // Keep at least one full bag buffered so `preview` can look ahead
            // across a bag boundary without special-casing.
            self.refill();
        }
        self.queue.pop_front().expect("bag refilled, never empty")
    }

    /// Peek at the next `n` pieces without consuming them.
    pub fn preview(&mut self, n: usize) -> Vec<PieceKind> {
        while self.queue.len() < n {
            self.refill();
        }
        self.queue.iter().take(n).copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_seven_draws_is_one_of_each() {
        let mut bag = SevenBag::new(7);
        for _ in 0..20 {
            let seven: HashSet<PieceKind> = (0..7).map(|_| bag.next()).collect();
            assert_eq!(seven.len(), 7, "each bag of 7 must contain every kind exactly once");
        }
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = SevenBag::new(123);
        let mut b = SevenBag::new(123);
        let seq_a: Vec<PieceKind> = (0..50).map(|_| a.next()).collect();
        let seq_b: Vec<PieceKind> = (0..50).map(|_| b.next()).collect();
        assert_eq!(seq_a, seq_b);
    }

    #[test]
    fn preview_matches_subsequent_draws() {
        let mut bag = SevenBag::new(99);
        let preview = bag.preview(5);
        let drawn: Vec<PieceKind> = (0..5).map(|_| bag.next()).collect();
        assert_eq!(preview, drawn);
    }
}
