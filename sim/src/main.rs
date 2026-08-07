//! Headless batch benchmark: plays N seeded Cross Tetris games to
//! completion (or a piece cap) using a rule-based agent, with no rendering.
//! Used to measure both AI strength (score/lines) and simulator throughput
//! (games/sec, placements/sec), and — via `--save-baseline`/`--check-baseline`
//! — to prove that a "pure speed" refactor doesn't change which placements
//! the AI chooses.
//!
//! Usage:
//!   cargo run -p sim --release -- [--seeds START..END] [--max-pieces N] [--save-baseline PATH] [--check-baseline PATH]
//!
//! Dev seeds (iterate freely): 1000..1100 (default).
//! Test seeds (touch once, for the final report only): 5000..5100.

use ai::{best_cross_placement, dellacherie, lookahead, play_best_cross_move, DEFAULT_WEIGHTS};
use engine::CrossGame;
use std::fs;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
enum Evaluator {
    Greedy,
    Dellacherie,
    Lookahead,
}

struct Args {
    seed_start: u64,
    seed_end: u64,
    max_pieces: u32,
    save_baseline: Option<String>,
    check_baseline: Option<String>,
    evaluator: Evaluator,
    beam_width: usize,
    per_game: bool,
}

fn parse_args() -> Args {
    let mut seed_start = 1000u64;
    let mut seed_end = 1100u64;
    let mut max_pieces = 20_000u32;
    let mut save_baseline = None;
    let mut check_baseline = None;
    let mut evaluator = Evaluator::Greedy;
    let mut beam_width = lookahead::DEFAULT_BEAM_WIDTH;
    let mut per_game = false;

    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--seeds" => {
                i += 1;
                let spec = argv.get(i).expect("--seeds requires START..END");
                let (a, b) = spec.split_once("..").expect("--seeds format is START..END");
                seed_start = a.parse().expect("bad seed start");
                seed_end = b.parse().expect("bad seed end");
            }
            "--max-pieces" => {
                i += 1;
                max_pieces = argv.get(i).expect("--max-pieces requires N").parse().expect("bad max-pieces");
            }
            "--save-baseline" => {
                i += 1;
                save_baseline = Some(argv.get(i).expect("--save-baseline requires PATH").clone());
            }
            "--check-baseline" => {
                i += 1;
                check_baseline = Some(argv.get(i).expect("--check-baseline requires PATH").clone());
            }
            "--evaluator" => {
                i += 1;
                evaluator = match argv.get(i).expect("--evaluator requires greedy|dellacherie|lookahead").as_str() {
                    "greedy" => Evaluator::Greedy,
                    "dellacherie" => Evaluator::Dellacherie,
                    "lookahead" => Evaluator::Lookahead,
                    other => panic!("unknown evaluator: {other}"),
                };
            }
            "--beam-width" => {
                i += 1;
                beam_width = argv.get(i).expect("--beam-width requires N").parse().expect("bad beam-width");
            }
            "--per-game" => {
                per_game = true;
            }
            other => panic!("unknown argument: {other}"),
        }
        i += 1;
    }

    Args { seed_start, seed_end, max_pieces, save_baseline, check_baseline, evaluator, beam_width, per_game }
}

#[derive(Clone, Copy, PartialEq)]
enum EndReason {
    ToppedOut,
    Capped,
    Stalled,
}

struct GameResult {
    seed: u64,
    score: u64,
    lines: u32,
    pieces: u32,
    end: EndReason,
}

/// Plays one game to completion, calling `play_best_cross_move` each step.
/// Returns the result plus, if `record` is set, the full placement sequence
/// (recomputed via `best_cross_placement` alongside the real move — only
/// done when recording, so it never affects the throughput measurement).
fn play_one_game(seed: u64, max_pieces: u32, record: bool, evaluator: Evaluator, beam_width: usize) -> (GameResult, Vec<String>) {
    let mut cross = CrossGame::new(seed);
    let mut pieces = 0u32;
    let mut placements = Vec::new();
    let end;
    loop {
        if cross.is_game_over() {
            end = EndReason::ToppedOut;
            break;
        }
        if pieces >= max_pieces {
            end = EndReason::Capped;
            break;
        }
        if record {
            let recorded = match evaluator {
                Evaluator::Greedy => best_cross_placement(&mut cross, &DEFAULT_WEIGHTS),
                Evaluator::Dellacherie => dellacherie::best_cross_placement(&mut cross, &dellacherie::DELLACHERIE_WEIGHTS),
                Evaluator::Lookahead => lookahead::best_cross_placement(&mut cross, &dellacherie::DELLACHERIE_WEIGHTS, beam_width),
            };
            if let Some(p) = recorded {
                placements.push(format!("{:?} {:?} {} {}", p.arm, p.rotation, p.column, p.row));
            }
        }
        let before = cross.total_pieces_placed();
        match evaluator {
            Evaluator::Greedy => play_best_cross_move(&mut cross, &DEFAULT_WEIGHTS),
            Evaluator::Dellacherie => dellacherie::play_best_cross_move(&mut cross, &dellacherie::DELLACHERIE_WEIGHTS),
            Evaluator::Lookahead => lookahead::play_best_cross_move(&mut cross, &dellacherie::DELLACHERIE_WEIGHTS, beam_width),
        }
        if cross.total_pieces_placed() == before {
            // No progress this step. Two distinct causes, and they must not
            // be conflated:
            //   1. select_well spawned the piece at that well's canonical
            //      (compensating) rotation and it didn't fit there — a
            //      legitimate top-out, same as standard Tetris rejecting a
            //      spawn regardless of whether some *other* rotation would
            //      have fit. cross.is_game_over() is already true here.
            //   2. best_cross_placement found no candidate at all across
            //      every selectable well for this piece, while no well is
            //      actually topped out — this would be a genuine stall (and,
            //      per is_well_selectable's invariant, shouldn't happen).
            // Without this progress check at all, case 2 spins forever.
            end = if cross.is_game_over() { EndReason::ToppedOut } else { EndReason::Stalled };
            break;
        }
        pieces += 1;
    }
    (
        GameResult { seed, score: cross.total_score(), lines: cross.total_lines_cleared(), pieces, end },
        placements,
    )
}

fn percentile<T: Copy + Default>(sorted: &[T], p: f64) -> T {
    if sorted.is_empty() {
        return T::default();
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn main() {
    let args = parse_args();
    let record = args.save_baseline.is_some() || args.check_baseline.is_some();

    let start = Instant::now();
    let mut results = Vec::new();
    let mut all_placements = Vec::new();

    for seed in args.seed_start..args.seed_end {
        let (result, placements) = play_one_game(seed, args.max_pieces, record, args.evaluator, args.beam_width);
        if record {
            all_placements.push(format!("# seed {seed}"));
            all_placements.extend(placements);
        }
        results.push(result);
    }
    let elapsed = start.elapsed();

    if let Some(path) = &args.save_baseline {
        fs::write(path, all_placements.join("\n") + "\n").expect("failed to write baseline");
        println!("Saved baseline placement sequence to {path}");
    }
    if let Some(path) = &args.check_baseline {
        let expected = fs::read_to_string(path).expect("failed to read baseline");
        let actual = all_placements.join("\n") + "\n";
        if expected == actual {
            println!("OK: placement sequence matches baseline exactly ({path})");
        } else {
            eprintln!("MISMATCH: placement sequence differs from baseline ({path})");
            let exp_lines: Vec<&str> = expected.lines().collect();
            let act_lines: Vec<&str> = actual.lines().collect();
            for (i, (e, a)) in exp_lines.iter().zip(act_lines.iter()).enumerate() {
                if e != a {
                    eprintln!("  first diff at line {i}:\n    baseline: {e}\n    actual:   {a}");
                    break;
                }
            }
            if exp_lines.len() != act_lines.len() {
                eprintln!("  line count differs: baseline={} actual={}", exp_lines.len(), act_lines.len());
            }
            std::process::exit(1);
        }
    }

    let games = results.len();
    let mut scores: Vec<u64> = results.iter().map(|r| r.score).collect();
    scores.sort_unstable();
    let mut lines: Vec<u32> = results.iter().map(|r| r.lines).collect();
    lines.sort_unstable();
    let total_pieces: u64 = results.iter().map(|r| r.pieces as u64).sum();

    let mean_score = scores.iter().map(|&s| s as f64).sum::<f64>() / games as f64;
    let mean_lines = lines.iter().map(|&l| l as f64).sum::<f64>() / games as f64;
    let mean_pieces = total_pieces as f64 / games as f64;

    let topped_out = results.iter().filter(|r| r.end == EndReason::ToppedOut).count();
    let capped = results.iter().filter(|r| r.end == EndReason::Capped).count();
    let stalled = results.iter().filter(|r| r.end == EndReason::Stalled).count();

    let evaluator_label = match args.evaluator {
        Evaluator::Greedy => "greedy".to_string(),
        Evaluator::Dellacherie => "dellacherie".to_string(),
        Evaluator::Lookahead => format!("lookahead (beam={})", args.beam_width),
    };
    let best = results.iter().max_by_key(|r| r.score);
    let worst = results.iter().min_by_key(|r| r.score);

    println!("evaluator:        {evaluator_label}");
    println!("games:            {games}");
    println!("max score:        {} (seed {})", best.map(|r| r.score).unwrap_or(0), best.map(|r| r.seed).unwrap_or(0));
    println!("mean score:       {mean_score:.1}");
    println!("median score:     {}", percentile(&scores, 0.5));
    println!("p10 score:        {}", percentile(&scores, 0.10));
    println!("min score:        {} (seed {})", worst.map(|r| r.score).unwrap_or(0), worst.map(|r| r.seed).unwrap_or(0));
    println!("mean lines:       {mean_lines:.1}");
    println!("mean pieces:      {mean_pieces:.1}");
    println!("topped out:       {topped_out}   capped: {capped}   stalled: {stalled}");
    println!(
        "throughput:       {:.1} games/sec   {:.0} placements/sec",
        games as f64 / elapsed.as_secs_f64(),
        total_pieces as f64 / elapsed.as_secs_f64()
    );
    println!("elapsed:          {:.3} s", elapsed.as_secs_f64());

    if args.per_game {
        let mut by_score: Vec<&GameResult> = results.iter().collect();
        by_score.sort_unstable_by_key(|r| std::cmp::Reverse(r.score));
        println!();
        println!("{:>10}  {:>12}  {:>8}  {:>8}  {:>10}", "seed", "score", "lines", "pieces", "end");
        for r in by_score {
            let end = match r.end {
                EndReason::ToppedOut => "topped_out",
                EndReason::Capped => "capped",
                EndReason::Stalled => "stalled",
            };
            println!("{:>10}  {:>12}  {:>8}  {:>8}  {:>10}", r.seed, r.score, r.lines, r.pieces, end);
        }
    }

    if capped > games / 2 {
        eprintln!("WARNING: over half the games hit --max-pieces ({max_pieces}) — scores are censored, raise the cap", max_pieces = args.max_pieces);
    }
}

#[cfg(test)]
mod debug_stall {
    use ai::{best_cross_placement, DEFAULT_WEIGHTS};
    use engine::{Action, CrossGame};

    #[test]
    fn diagnose_stall_seed_1000() {
        let mut cross = CrossGame::new(1000);
        let mut pieces = 0u32;
        loop {
            if cross.is_game_over() || pieces >= 20000 {
                println!("ended normally after {pieces} pieces, game_over={}", cross.is_game_over());
                return;
            }
            let Some(placement) = best_cross_placement(&mut cross, &DEFAULT_WEIGHTS) else {
                println!("best_cross_placement returned None after {pieces} pieces");
                for arm in engine::Arm::ALL {
                    println!(
                        "  {:?}: game_over={} pieces_placed={} selectable={}",
                        arm,
                        cross.well(arm).game_over,
                        cross.well(arm).pieces_placed,
                        cross.is_well_selectable(arm)
                    );
                }
                return;
            };
            let before_active = cross.awaiting_well_selection();
            assert!(before_active);
            let selected = cross.select_well(placement.arm);
            if !selected {
                println!("select_well({:?}) FAILED after {pieces} pieces, was selectable per best_cross_placement", placement.arm);
                return;
            }
            let Some(active) = cross.active_piece() else {
                println!(
                    "select_well({:?}) succeeded but active_piece() is None after {pieces} pieces -> immediate top-out at spawn",
                    placement.arm
                );
                println!("  well.game_over = {}", cross.well(placement.arm).game_over);
                println!("  spawn rotation would have been the arm's compensating rotation");
                println!("  AI wanted rotation={:?} col={} row={}", placement.rotation, placement.column, placement.row);
                return;
            };
            cross.force_active_placement(placement.rotation, placement.row, placement.column);
            cross.apply(Action::HardDrop);
            pieces += 1;
        }
    }
}
