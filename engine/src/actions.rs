/// Real-time actions the engine accepts. The engine has no notion of frames or
/// wall-clock time itself; `Tick` carries elapsed milliseconds from the caller
/// so gravity/lock-delay timers stay deterministic and testable without a
/// real clock.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Action {
    MoveLeft,
    MoveRight,
    RotateCw,
    RotateCcw,
    SoftDropStart,
    SoftDropEnd,
    HardDrop,
    Hold,
    Tick(f64),
}
