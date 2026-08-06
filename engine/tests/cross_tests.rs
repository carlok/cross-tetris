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
fn tick_and_actions_are_no_ops_while_awaiting_selection() {
    let mut cross = CrossGame::new(5);
    let before = cross.clone();
    cross.apply(Action::Tick(5000.0));
    cross.apply(Action::MoveLeft);
    cross.apply(Action::HardDrop);
    assert_eq!(cross, before, "nothing should happen until a well is selected");
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
