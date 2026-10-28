//! The system's reduce motion setting reaches every animation, those already running included.
//! A test binary of its own: the setting is process-wide.

use opendap_motion::animator::{Animator, Kind};
use opendap_motion::spring::Spring;
use opendap_motion::transition::{End, Transition};

#[test]
fn the_system_setting_lands_every_animation_at_once() {
    let mut thrown = Animator::new(100.0, Kind::Spring(Spring::SETTLE));
    thrown.retarget(0.0, 0.0, Some(800.0));
    assert!(thrown.running(0.05), "moving while motion is on");
    let mut t = Transition::new(End::From, 320.0);
    t.go(End::To, 0.0);
    opendap_motion::set_reduce_motion(true);
    assert!(
        !thrown.running(0.05),
        "the one already running lands at once"
    );
    assert_eq!(thrown.sample(0.05).0, 0.0);
    assert_eq!(t.progress(0.05), 1.0, "a transition too");
    let mut new = Animator::new(5.0, Kind::Spring(Spring::SETTLE));
    new.retarget(9.0, 0.0, None);
    assert_eq!(
        new.sample(0.0).0,
        9.0,
        "one made without saying so honours it"
    );
    opendap_motion::set_reduce_motion(false);
    assert!(new.running(0.01) || new.sample(0.01).0 == 9.0);
}
