//! Motion primitives (ui-model §8): what moves and how, as pure functions of time, so a
//! component animates the same on the T1, on OpenDAP Desktop and in a test with a virtual clock.
//!
//! - [`spring`]: springs for what a hand moves, carrying the gesture's velocity; critically
//!   damped by default (no overshoot), bounce only where asked.
//! - [`easing`]: the motion tokens' curves (cubic-bezier, solved exactly), for what the system
//!   starts.
//! - [`decay`]: a fling's deceleration, and where it stops.
//! - [`animator`]: an interruptible animation: a new target takes over from wherever it is, with
//!   its velocity; reduce motion settles at once.
//! - [`transition`]: a transition's progress, driven by time or by a hand, settling on the end a
//!   fling meant, reversible and interruptible.
//! - [`interpolate`]: what moves with it: numbers, a shared element's frame, colours in linear
//!   light, a container transform's fade through.
//! - [`presence`]: enter and exit: what is removed stays drawn while it leaves.
//! - [`budget`]: frames only while something moves (0 wakeups at rest, composer spec §5.1).
//!
//! **Reduce motion** (DS-012) is the system's setting: the application sets it here once, from the
//! system, and every animation honours it, those already running included ([`set_reduce_motion`]).
//! An animation's own `reduce_motion` can only add to it (a screen that wants one animation
//! instant), never take it away: no caller can forget it.

pub mod animator;
pub mod budget;
pub mod decay;
pub mod easing;
pub mod interpolate;
pub mod presence;
pub mod spring;
pub mod transition;

use std::sync::atomic::{AtomicBool, Ordering};

static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);

/// The system's reduce motion setting, as the application receives it (and again when it changes).
pub fn set_reduce_motion(on: bool) {
    REDUCE_MOTION.store(on, Ordering::Relaxed);
}

/// Whether the system asks for reduced motion.
pub fn reduce_motion() -> bool {
    REDUCE_MOTION.load(Ordering::Relaxed)
}
