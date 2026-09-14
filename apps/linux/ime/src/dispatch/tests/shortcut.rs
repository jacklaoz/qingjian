//! 「修饰键 + 数字」快捷键：删候选、上屏译词。

use super::*;

/// 缺省删候选键是 Shift + 数字，而 Shift + 1 的 keysym 是 `!`：
/// 必须靠键位认出数字，否则这条快捷键在 Linux 上整个失效。
#[test]
fn shift_digit_deletes_a_candidate_by_keycode() {
    let mut router = router();
    assert_eq!(
        chord(SHIFT),
        router.config().delete_keys,
        "缺省删候选键就是 Shift"
    );
    router.type_str("ni");
    let first = router.page_texts()[0].clone();
    // Shift + 1：字符是 `!`，键位是数字行第一个
    let response = router.handle_key(KeyInput::new('!' as u32, KEYCODE_1, SHIFT));
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    let notice = router.current_frame().notice.expect("有一句反馈");
    assert!(
        notice.contains(&first),
        "反馈里提到被删的词，实际 {notice:?}"
    );
}

/// 提示只活到下一次按键。
#[test]
fn notice_lives_until_the_next_key() {
    let mut router = router();
    router.type_str("ni");
    router.handle_key(KeyInput::new('!' as u32, KEYCODE_1, SHIFT));
    assert!(router.current_frame().notice.is_some());
    router.press('h');
    assert!(router.current_frame().notice.is_none(), "下一键就清掉");
}

/// 没在组句时 Shift + 数字不是快捷键，走中文标点（⇧1 出感叹号）。
#[test]
fn shift_digit_is_punctuation_when_idle() {
    let mut router = router();
    let response = router.handle_key(KeyInput::new('!' as u32, KEYCODE_1, SHIFT));
    assert_eq!(response.commit.as_deref(), Some("！"), "全角感叹号");
}

/// 表达式模式里 Shift + 数字打的是运算符，不当快捷键。
#[test]
fn expression_mode_ignores_digit_shortcuts() {
    let mut router = router();
    router.type_str("v1");
    // Shift + 8 在 US 布局上是 `*`
    let response = router.handle_key(KeyInput::new('*' as u32, 17, SHIFT));
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert!(router.current_frame().notice.is_none(), "没被当成删候选");
}

/// 配到译词键但候选没有译文时吞掉按键、什么都不做（样例词库没装释义表）。
#[test]
fn translation_shortcut_without_a_gloss_does_nothing() {
    let mut router = router();
    let translation = router.config().translation_keys.0;
    router.type_str("ni");
    let before = router.page_texts();
    let state = if translation.alt { ALT } else { CONTROL };
    let response = router.handle_key(KeyInput::new('1' as u32, KEYCODE_1, state));
    assert_eq!(response.outcome, KeyOutcome::Consumed, "吞掉，不交给应用");
    assert!(response.commit.is_none(), "没有译文就不上屏");
    assert_eq!(router.page_texts(), before, "候选没动");
}
