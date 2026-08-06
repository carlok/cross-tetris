use engine::{Action, GameState};

fn fixed_action_log() -> Vec<Action> {
    vec![
        Action::Tick(16.0),
        Action::MoveLeft,
        Action::Tick(16.0),
        Action::RotateCw,
        Action::Tick(16.0),
        Action::MoveRight,
        Action::MoveRight,
        Action::Tick(16.0),
        Action::SoftDropStart,
        Action::Tick(200.0),
        Action::SoftDropEnd,
        Action::Hold,
        Action::Tick(16.0),
        Action::RotateCcw,
        Action::HardDrop,
        Action::Tick(600.0),
        Action::MoveLeft,
        Action::HardDrop,
    ]
}

#[test]
fn replaying_same_seed_and_action_log_reproduces_identical_state() {
    let seed = 0xC0FFEE;
    let mut a = GameState::new(seed);
    let mut b = GameState::new(seed);

    for action in fixed_action_log() {
        a.apply(action);
        b.apply(action);
    }

    assert_eq!(a, b, "same seed + same action log must produce identical state");
}

#[test]
fn different_seeds_typically_diverge() {
    let mut a = GameState::new(1);
    let mut b = GameState::new(2);
    for action in fixed_action_log() {
        a.apply(action);
        b.apply(action);
    }
    assert_ne!(a.board, b.board, "different seeds should (almost always) produce different boards");
}
