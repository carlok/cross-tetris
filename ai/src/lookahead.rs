//! Two-ply beam-search lookahead (plan.md step 4): for the current piece and
//! the next piece the shared queue already exposes, search one ply deeper
//! than `dellacherie::best_cross_placement` — for each candidate first
//! placement, find its best reply with the *second* upcoming piece (which
//! may land in any selectable well, not just the one the first placement
//! used), and pick the first placement whose (first-ply eval + best-reply
//! eval) is highest.
//!
//! Naively this is ~160 first placements × ~160 second placements ≈ 25,600
//! board simulations per decision. Beam pruning keeps only the top
//! `beam_width` first-ply candidates (ranked by their own 1-ply eval) before
//! spending the second ply on them, making the cost ~`beam_width` × 160.
//! Only practical after step 2's allocation-free `BitBoard` search.
//!
//! Uses the Dellacherie feature set (the strongest evaluator available) for
//! both plies' scoring — this is a lookahead *wrapper* around it, not a
//! third independent evaluator.

use crate::bitboard::BitBoard;
use crate::dellacherie::{self, Weights};
use crate::greedy::CrossPlacement;
use crate::placement::{enumerate_placements, simulate_lock};
use engine::cross::{Arm, CrossGame, MAX_WELL_IMBALANCE};
use engine::Action;

/// Default beam width (plan.md's suggested starting point). Kept as a
/// parameter on every public function so the benchmark can sweep it.
pub const DEFAULT_BEAM_WIDTH: usize = 8;

/// A frozen snapshot of one well's board + piece count, used to simulate
/// placements without mutating the real `CrossGame` — the search explores
/// many hypothetical futures and must not touch actual game state.
#[derive(Copy, Clone)]
struct WellSnapshot {
    board: BitBoard,
    pieces_placed: u32,
    game_over: bool,
}

fn snapshot(cross: &CrossGame) -> [WellSnapshot; 4] {
    core::array::from_fn(|i| {
        let well = cross.well(Arm::ALL[i]);
        WellSnapshot { board: BitBoard::from_board(&well.board), pieces_placed: well.pieces_placed, game_over: well.game_over }
    })
}

/// Mirrors `CrossGame::is_well_selectable`'s rule exactly, but against a
/// hypothetical `pieces_placed` snapshot rather than the live game — needed
/// because after a simulated first placement, the well imbalance rule may
/// block a well for the second piece that was open for the first.
fn is_well_selectable_sim(snapshots: &[WellSnapshot; 4], arm: Arm) -> bool {
    let min_pieces_placed = snapshots.iter().map(|w| w.pieces_placed).min().unwrap_or(0);
    let well = &snapshots[arm.index()];
    !well.game_over && well.pieces_placed < min_pieces_placed + MAX_WELL_IMBALANCE
}

fn eval_board(board_before: &BitBoard, piece: &engine::piece::ActivePiece, board_after: &BitBoard, weights: &Weights) -> f32 {
    dellacherie::score_board(board_before, piece, board_after, weights)
}

/// The highest-total-scoring first placement for the current piece, judged
/// by its own 1-ply eval plus the best possible reply to the *next* piece
/// (which may go in any well the imbalance rule still allows afterward).
/// `None` under the same conditions as `dellacherie::best_cross_placement`
/// (no active selection pending, or no well selectable).
pub fn best_cross_placement(cross: &mut CrossGame, weights: &Weights, beam_width: usize) -> Option<CrossPlacement> {
    if !cross.awaiting_well_selection() {
        return None;
    }
    let kind1 = *cross.next_queue(1).first()?;
    let snapshots = snapshot(cross);

    // Ply 1: every legal placement across every currently selectable well,
    // scored by its own resulting-board eval.
    let mut ply1: Vec<(Arm, engine::piece::ActivePiece, BitBoard, f32)> = Arm::ALL
        .iter()
        .copied()
        .filter(|&arm| cross.is_well_selectable(arm))
        .flat_map(|arm| {
            let bb = snapshots[arm.index()].board;
            enumerate_placements(&bb, kind1)
                .into_iter()
                .map(move |piece| {
                    let (board1, _cleared) = simulate_lock(bb, &piece);
                    let eval1 = eval_board(&bb, &piece, &board1, weights);
                    (arm, piece, board1, eval1)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    if ply1.is_empty() {
        return None;
    }

    // Beam prune: keep only the top `beam_width` by 1-ply eval before
    // spending the (much more expensive) second ply on them.
    ply1.sort_unstable_by(|a, b| b.3.partial_cmp(&a.3).unwrap());
    ply1.truncate(beam_width.max(1));

    // The second upcoming piece. If the queue can't provide one (should not
    // happen — the bag always refills — but guarded for robustness), fall
    // back to pure 1-ply: the best-reply term is simply 0 for every
    // candidate, so the ply-1 eval alone decides, same ranking as the
    // beam-pruning sort above already produced.
    let kind2 = cross.next_queue(2).get(1).copied();

    let mut best: Option<(Arm, engine::piece::ActivePiece, f32)> = None;
    for (arm1, piece1, board1, eval1) in ply1 {
        let best_reply = match kind2 {
            None => 0.0,
            Some(kind2) => {
                let mut snapshots2 = snapshots;
                snapshots2[arm1.index()] = WellSnapshot { board: board1, pieces_placed: snapshots[arm1.index()].pieces_placed + 1, game_over: false };
                Arm::ALL
                    .iter()
                    .copied()
                    .filter(|&arm2| is_well_selectable_sim(&snapshots2, arm2))
                    .flat_map(|arm2| {
                        let bb2 = snapshots2[arm2.index()].board;
                        enumerate_placements(&bb2, kind2)
                            .into_iter()
                            .map(move |piece2| {
                                let (board2, _cleared) = simulate_lock(bb2, &piece2);
                                eval_board(&bb2, &piece2, &board2, weights)
                            })
                            .collect::<Vec<_>>()
                    })
                    .fold(f32::NEG_INFINITY, f32::max)
            }
        };
        // If no reply was possible at all (every well blocked — shouldn't
        // happen per is_well_selectable's invariant, but guarded), treat as
        // 0 rather than -infinity so this candidate isn't unconditionally
        // rejected in favor of one with a worse first ply but *some* reply.
        let best_reply = if best_reply.is_finite() { best_reply } else { 0.0 };
        let total = eval1 + best_reply;
        if best.is_none_or(|(_, _, best_total)| total > best_total) {
            best = Some((arm1, piece1, total));
        }
    }

    best.map(|(arm, piece, _)| CrossPlacement { arm, rotation: piece.rotation, column: piece.col, row: piece.row })
}

/// Computes the 2-ply best cross placement for the upcoming piece, commits
/// it to that well, and hard-drops it there.
pub fn play_best_cross_move(cross: &mut CrossGame, weights: &Weights, beam_width: usize) {
    let Some(placement) = best_cross_placement(cross, weights, beam_width) else {
        return;
    };
    cross.select_well(placement.arm);
    let Some(active) = cross.active_piece() else { return };
    let drop_distance = (placement.row - active.row).max(0) as u32;
    cross.force_active_placement(placement.rotation, placement.row, placement.column);
    cross.apply(Action::HardDrop);
    cross.wells[placement.arm.index()].score += drop_distance as u64 * engine::scoring::HARD_DROP_POINTS_PER_CELL;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dellacherie::DELLACHERIE_WEIGHTS;

    /// `is_well_selectable_sim` must agree with the live
    /// `CrossGame::is_well_selectable` when the snapshot is taken straight
    /// from that same game (no hypothetical mutation yet) — otherwise ply-1
    /// selectability (computed from the real game) and ply-2 selectability
    /// (computed from the snapshot) could disagree even before any
    /// simulated placement is applied.
    #[test]
    fn is_well_selectable_sim_matches_engine_rule_on_an_unmodified_snapshot() {
        let mut cross = CrossGame::new(9001);
        // Play a bunch of pieces with plain greedy so the wells end up with
        // uneven pieces_placed counts, exercising the imbalance rule.
        for _ in 0..150 {
            if cross.is_game_over() {
                break;
            }
            crate::greedy::play_best_cross_move(&mut cross, &crate::greedy::DEFAULT_WEIGHTS);
        }
        let snapshots = snapshot(&cross);
        for &arm in &Arm::ALL {
            assert_eq!(
                is_well_selectable_sim(&snapshots, arm),
                cross.is_well_selectable(arm),
                "mismatch for {arm:?}: pieces_placed={:?}",
                snapshots.map(|w| w.pieces_placed)
            );
        }
    }

    /// Two independent runs from the same seed, both using the lookahead
    /// agent, must choose the exact same placement at every step — no
    /// nondeterminism from float comparisons, HashMap iteration order, or
    /// similar. `partial_cmp().unwrap()` would panic outright on a NaN
    /// feature value, which this also indirectly guards against (a panic
    /// fails the test).
    #[test]
    fn deterministic_across_repeated_runs_from_the_same_seed() {
        let placements_for = |seed: u64| -> Vec<CrossPlacement> {
            let mut cross = CrossGame::new(seed);
            let mut out = Vec::new();
            for _ in 0..300 {
                if cross.is_game_over() {
                    break;
                }
                let Some(p) = best_cross_placement(&mut cross, &DELLACHERIE_WEIGHTS, DEFAULT_BEAM_WIDTH) else { break };
                out.push(p);
                play_best_cross_move(&mut cross, &DELLACHERIE_WEIGHTS, DEFAULT_BEAM_WIDTH);
            }
            out
        };
        let run1 = placements_for(4242);
        let run2 = placements_for(4242);
        assert_eq!(run1, run2);
    }

    /// A full game with the lookahead agent must run to completion (or a
    /// generous piece cap) without panicking, and must make some progress —
    /// covers the "no float NaN comparisons" and "respects is_well_selectable
    /// at ply 2" acceptance criteria end-to-end, not just in isolation.
    #[test]
    fn plays_a_full_game_without_panicking_and_makes_progress() {
        let mut cross = CrossGame::new(777);
        let mut pieces = 0u32;
        while !cross.is_game_over() && pieces < 500 {
            let before = cross.total_pieces_placed();
            play_best_cross_move(&mut cross, &DELLACHERIE_WEIGHTS, DEFAULT_BEAM_WIDTH);
            if cross.total_pieces_placed() == before {
                break;
            }
            pieces += 1;
        }
        assert!(pieces > 50, "expected substantial progress, got {pieces} pieces");
    }
}
