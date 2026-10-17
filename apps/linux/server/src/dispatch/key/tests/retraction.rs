//! 不在组句时的退格告诉 Core：上屏的词整个删掉再重打选别的，那次的学习退回去。
use super::support::{compose, key};
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_learning::FrequencyLearner;
use qingjian_platform::protocol::{
    ClientMessage, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, SessionId,
};

const BACK: u32 = 0x08;

fn router() -> Router {
    let engine = Engine::new(Dictionary::parse("腿\ttui\t200\n推\ttui\t100\n").unwrap())
        .with_learner(Box::new(FrequencyLearner::default()));
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    // 会话开着时缺省按隐私处理（不学习），前端同步过输入框的能力才放开
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: false,
    });
    router
}

#[test]
fn erasing_a_commit_and_picking_another_word_retracts_its_learning() {
    let normal = KeyModifiers::default();
    let mut router = router();
    compose(&mut router, "tui", normal);
    assert_eq!(
        key(&mut router, 32, Some(' '), normal).1.as_deref(),
        Some("腿")
    );
    assert_eq!(router.engine.learner().weight("腿"), 1);
    // 退格交给应用删字，Core 记下刚上屏的「腿」没了
    assert_eq!(
        key(&mut router, BACK, None, normal).0,
        KeyOutcome::Passthrough
    );
    compose(&mut router, "tui", normal);
    assert_eq!(
        key(&mut router, 0x32, Some('2'), normal).1.as_deref(),
        Some("推")
    );
    assert_eq!(router.engine.learner().weight("腿"), 0);
    assert_eq!(router.engine.learner().choice_weight("tui", "腿"), 0);
    assert_eq!(router.engine.learner().weight("推"), 1);
}

#[test]
fn word_wise_backspace_does_not_retract() {
    let normal = KeyModifiers::default();
    let ctrl = KeyModifiers {
        ctrl: true,
        ..Default::default()
    };
    let mut router = router();
    compose(&mut router, "tui", normal);
    key(&mut router, 32, Some(' '), normal);
    // Ctrl+退格删了多少字说不准：不当成删掉了这次上屏
    assert_eq!(
        key(&mut router, BACK, None, ctrl).0,
        KeyOutcome::Passthrough
    );
    compose(&mut router, "tui", normal);
    key(&mut router, 0x32, Some('2'), normal);
    assert_eq!(router.engine.learner().weight("腿"), 1);
}
