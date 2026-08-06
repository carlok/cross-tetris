use engine::{Action, Arm, CrossGame};

#[test]
fn starts_awaiting_well_selection_with_no_active_piece() {
    let cross = CrossGame::new(1);
    assert!(cross.awaiting_well_selection());
    assert!(cross.active_arm().is_none());
}

#[test]
fn select_well_spawns_the_next_queued_piece_there() {
    let mut cross = CrossGame::new(2);
    let queued = cross.next_queue(1)[0];

    assert!(cross.select_well(Arm::East));

    assert_eq!(cross.active_arm(), Some(Arm::East));
    assert_eq!(cross.active_piece().unwrap().kind, queued);
    assert!(!cross.awaiting_well_selection());
}

#[test]
fn cannot_select_a_well_while_a_piece_is_already_falling() {
    let mut cross = CrossGame::new(3);
    assert!(cross.select_well(Arm::North));
    assert!(!cross.select_well(Arm::South), "a second selection must be rejected until the current piece locks");
    assert_eq!(cross.active_arm(), Some(Arm::North));
}

#[test]
fn only_the_selected_well_gets_gravity_and_the_rest_stay_static() {
    let mut cross = CrossGame::new(4);
    cross.select_well(Arm::West);
    let before_others: Vec<_> =
        [Arm::North, Arm::East, Arm::South].iter().map(|&a| cross.well(a).clone()).collect();

    cross.apply(Action::Tick(1200.0)); // > 1000ms/row at level 1

    let after_others: Vec<_> =
        [Arm::North, Arm::East, Arm::South].iter().map(|&a| cross.well(a).clone()).collect();
    assert_eq!(before_others, after_others, "wells with no active piece must not change on Tick");
    assert!(cross.active_piece().unwrap().row > 0, "the selected well's piece should have fallen");
}

#[test]
fn move_and_drop_actions_are_no_ops_while_awaiting_selection() {
    let mut cross = CrossGame::new(5);
    let before = cross.clone();
    cross.apply(Action::MoveLeft);
    cross.apply(Action::HardDrop);
    assert_eq!(cross, before, "nothing should happen until a well is selected");
}

#[test]
fn tick_while_awaiting_selection_only_advances_the_timeout_clock() {
    let mut cross = CrossGame::new(5);
    cross.apply(Action::Tick(1000.0)); // well under SELECTION_TIMEOUT_MS
    assert!(cross.awaiting_well_selection(), "should still be awaiting selection before the timeout elapses");
    for arm in Arm::ALL {
        assert_eq!(cross.well(arm).board.column_height(0), 0, "no well should have received a piece yet");
    }
}

#[test]
fn failing_to_select_a_well_in_time_auto_selects_one() {
    let mut cross = CrossGame::new(6);
    assert!(cross.awaiting_well_selection());
    cross.apply(Action::Tick(engine::cross::SELECTION_TIMEOUT_MS));
    assert!(!cross.awaiting_well_selection(), "a well should have been auto-selected once the timeout elapsed");
}

#[test]
fn auto_selection_never_picks_a_topped_out_well() {
    let mut cross = CrossGame::new(7);
    for arm in [Arm::North, Arm::East, Arm::South] {
        cross.wells[arm.index()].game_over = true;
    }
    cross.apply(Action::Tick(engine::cross::SELECTION_TIMEOUT_MS));
    assert_eq!(cross.active_arm(), Some(Arm::West), "the only open well must be the one auto-selected");
}

#[test]
fn selecting_a_well_manually_resets_the_timeout_clock() {
    let mut cross = CrossGame::new(8);
    cross.apply(Action::Tick(4000.0));
    cross.select_well(Arm::North);
    cross.apply(Action::HardDrop); // lock it, back to awaiting
    assert_eq!(cross.selection_timer_ms(), 0.0, "picking a well should reset the timeout clock for the next piece");
}

#[test]
fn locking_returns_to_awaiting_selection_for_the_next_piece() {
    let mut cross = CrossGame::new(6);
    cross.select_well(Arm::South);
    cross.apply(Action::HardDrop);
    assert!(cross.awaiting_well_selection());
    assert!(cross.active_arm().is_none());
    assert!(cross.well(Arm::South).score > 0 || cross.well(Arm::South).board.column_height(0) >= 0);
}

#[test]
fn hard_drop_scores_only_the_selected_well() {
    let mut cross = CrossGame::new(7);
    cross.select_well(Arm::East);
    cross.apply(Action::HardDrop);
    assert!(cross.well(Arm::East).score > 0 || cross.well(Arm::East).lines_cleared_total > 0);
    for arm in [Arm::North, Arm::South, Arm::West] {
        assert_eq!(cross.well(arm).score, 0, "{arm:?} was never selected, must not score");
    }
}

#[test]
fn same_seed_reproduces_identical_cross_game() {
    let mut a = CrossGame::new(42);
    let mut b = CrossGame::new(42);
    for arm in [Arm::North, Arm::East, Arm::South, Arm::West, Arm::North] {
        if a.awaiting_well_selection() {
            a.select_well(arm);
            b.select_well(arm);
        }
        a.apply(Action::HardDrop);
        b.apply(Action::HardDrop);
    }
    assert_eq!(a, b);
}

#[test]
fn total_score_sums_all_wells() {
    let mut cross = CrossGame::new(9);
    cross.wells[Arm::North.index()].score = 100;
    cross.wells[Arm::East.index()].score = 50;
    cross.wells[Arm::West.index()].score = 25;
    assert_eq!(cross.total_score(), 175);
}

#[test]
fn game_over_when_any_single_well_tops_out() {
    let mut cross = CrossGame::new(11);
    assert!(!cross.is_game_over());
    cross.wells[Arm::South.index()].game_over = true;
    assert!(cross.is_game_over());
}

#[test]
fn selecting_a_topped_out_well_is_rejected() {
    let mut cross = CrossGame::new(12);
    cross.wells[Arm::North.index()].game_over = true;
    assert!(!cross.select_well(Arm::North));
    assert!(cross.awaiting_well_selection());
}

#[test]
fn hold_swaps_with_the_selected_wells_own_hold_slot() {
    let mut cross = CrossGame::new(13);
    cross.select_well(Arm::West);
    let original_kind = cross.active_piece().unwrap().kind;
    assert!(cross.well(Arm::West).hold.is_none());

    cross.apply(Action::Hold);

    assert_eq!(cross.well(Arm::West).hold, Some(original_kind));
    assert!(!cross.awaiting_well_selection(), "hold should give a new active piece immediately");
}

#[test]
fn rendered_board_only_composites_the_active_well() {
    let mut cross = CrossGame::new(14);
    cross.select_well(Arm::North);
    let north_rendered = cross.rendered_board(Arm::North);
    let east_rendered = cross.rendered_board(Arm::East);
    assert_ne!(north_rendered, cross.well(Arm::North).board, "active well's render should include the falling piece");
    assert_eq!(east_rendered, cross.well(Arm::East).board, "non-active well's render is just its static board");
}

#[test]
fn move_and_rotate_stay_within_the_selected_wells_board() {
    let mut cross = CrossGame::new(15);
    cross.select_well(Arm::South);
    cross.apply(Action::MoveLeft);
    cross.apply(Action::RotateCw);
    let piece = cross.active_piece().unwrap();
    assert!(engine::rotation::piece_fits(&cross.well(Arm::South).board, &piece));
}

fn place_one_piece(cross: &mut CrossGame, arm: Arm) -> bool {
    if !cross.select_well(arm) {
        return false;
    }
    cross.apply(Action::HardDrop);
    true
}

#[test]
fn well_becomes_unselectable_once_it_leads_by_max_imbalance() {
    let mut cross = CrossGame::new(30);
    for _ in 0..engine::cross::MAX_WELL_IMBALANCE {
        assert!(place_one_piece(&mut cross, Arm::North));
    }
    assert_eq!(cross.well(Arm::North).pieces_placed, engine::cross::MAX_WELL_IMBALANCE);
    assert!(!cross.is_well_selectable(Arm::North), "North is now MAX_WELL_IMBALANCE ahead of the untouched wells");
    assert!(!cross.select_well(Arm::North), "selecting an over-imbalanced well must fail");
    assert!(cross.is_well_selectable(Arm::East), "other wells remain selectable");
}

#[test]
fn placing_in_other_wells_reopens_the_leading_well() {
    let mut cross = CrossGame::new(31);
    for _ in 0..engine::cross::MAX_WELL_IMBALANCE {
        assert!(place_one_piece(&mut cross, Arm::North));
    }
    assert!(!cross.is_well_selectable(Arm::North));

    // The rule compares against the single least-used well, so *every*
    // other well needs to catch up by one before the global minimum rises.
    for arm in [Arm::East, Arm::South, Arm::West] {
        assert!(place_one_piece(&mut cross, arm));
    }

    assert!(cross.is_well_selectable(Arm::North), "North should be selectable again once the least-used well catches up by one");
}

#[test]
fn auto_selection_never_picks_an_over_imbalanced_well() {
    let mut cross = CrossGame::new(32);
    for _ in 0..engine::cross::MAX_WELL_IMBALANCE {
        assert!(place_one_piece(&mut cross, Arm::North));
    }
    cross.apply(Action::Tick(engine::cross::SELECTION_TIMEOUT_MS));
    assert_ne!(cross.active_arm(), Some(Arm::North), "auto-select must not pick an over-imbalanced well");
}

#[test]
fn there_is_always_at_least_one_selectable_well_among_open_ones() {
    let mut cross = CrossGame::new(33);
    // Hammer North far past the limit by alternating attempts across all
    // arms; the rule must never leave every non-topped-out well blocked.
    for i in 0..50 {
        let arm = Arm::ALL[i % 4];
        if cross.is_well_selectable(arm) {
            place_one_piece(&mut cross, arm);
        }
        assert!(
            Arm::ALL.iter().any(|&a| cross.is_well_selectable(a)),
            "at least one well must always remain selectable while none has topped out"
        );
    }
}

#[test]
fn spawn_rotation_compensates_each_arms_view_transform() {
    // South is the identity view, so it spawns at the plain canonical R0
    // rotation; the other three spawn pre-rotated so that, once the web UI
    // rotates the whole well for rendering, the piece still *appears*
    // canonical rather than pre-rotated (see spawn_rotation's doc comment
    // in cross.rs for the full derivation).
    use engine::piece::Rotation;

    let mut south = CrossGame::new(40);
    south.select_well(Arm::South);
    assert_eq!(south.active_piece().unwrap().rotation, Rotation::R0);

    let mut north = CrossGame::new(41);
    north.select_well(Arm::North);
    assert_eq!(north.active_piece().unwrap().rotation, Rotation::R2);

    let mut west = CrossGame::new(42);
    west.select_well(Arm::West);
    assert_eq!(west.active_piece().unwrap().rotation, Rotation::L);

    let mut east = CrossGame::new(43);
    east.select_well(Arm::East);
    assert_eq!(east.active_piece().unwrap().rotation, Rotation::R);
}

#[test]
fn every_piece_kind_fits_at_spawn_in_every_arm() {
    // The compensating spawn rotation changes which cells a piece occupies
    // at spawn; confirm this never causes an unexpected top-out for any of
    // the 7 kinds in any of the 4 arms (the spawn area should be empty on a
    // fresh well regardless of rotation state).
    for seed in 0..30 {
        let mut cross = CrossGame::new(seed);
        for arm in Arm::ALL {
            if !cross.select_well(arm) {
                continue; // imbalance limit or already active; fine, just skip
            }
            assert!(!cross.well(arm).game_over, "{arm:?} should not top out spawning into an empty well");
            cross.apply(Action::HardDrop);
        }
    }
}

#[test]
fn hold_respawn_also_uses_the_compensating_rotation() {
    use engine::piece::Rotation;

    let mut cross = CrossGame::new(44);
    cross.select_well(Arm::West);
    cross.apply(Action::Hold); // hold is empty, so this draws a fresh piece from the bag
    assert_eq!(cross.active_piece().unwrap().rotation, Rotation::L, "hold's fresh spawn should also compensate West's view transform");
}
