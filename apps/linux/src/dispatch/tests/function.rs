//! 功能键与模式键：没在组句时的放行、问字模式、小键盘、Shift+Tab。

use super::*;

/// 没在组句时功能键一律交还应用。
#[test]
fn function_keys_pass_through_when_idle() {
    let mut router = router();
    for key in [
        keysym::BACKSPACE,
        keysym::RETURN,
        keysym::ESCAPE,
        keysym::LEFT,
        keysym::UP,
        keysym::PAGE_DOWN,
        keysym::TAB,
    ] {
        assert_eq!(
            router.press_key(key).outcome,
            KeyOutcome::Passthrough,
            "keysym {key:#x} 该放行"
        );
    }
}

/// 中文模式的 Tab 没有整句补全时交还应用（缩进 / 跳焦点）。
#[test]
fn tab_passes_through_without_a_sentence_completion() {
    let mut router = router();
    router.type_str("ni");
    assert_eq!(
        router.press_key(keysym::TAB).outcome,
        KeyOutcome::Passthrough
    );
}

/// 小键盘的方向键与主键盘区等价。
#[test]
fn keypad_navigation_works_the_same() {
    let mut router = router();
    router.type_str("ni");
    router.press_key(keysym::KP_DOWN);
    assert_eq!(router.current_frame().highlight, 1, "小键盘下键也挪高亮");
}

/// 小键盘回车与主键盘回车一样，原样上屏。
#[test]
fn keypad_enter_commits_raw() {
    let mut router = router();
    router.type_str("nihao");
    assert_eq!(
        router.press_key(keysym::KP_ENTER).commit.as_deref(),
        Some("nihao")
    );
}

/// 缓冲为空时敲 `?` 进问字模式，preedit 里留着那个问号。
#[test]
fn question_prefix_enters_question_mode() {
    let mut router = router();
    let response = router.press('?');
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert!(response.commit.is_none(), "问号先不上屏");
    assert!(!router.current_frame().is_empty(), "进了组句");
}

/// 只有一个 `?` 时按回车：还原成问号上屏，别把消息发出去。
#[test]
fn bare_question_restores_on_return() {
    let mut router = router();
    router.press('?');
    let response = router.press_key(keysym::RETURN);
    assert_eq!(response.outcome, KeyOutcome::Consumed, "回车被吃掉");
    let commit = response.commit.expect("有上屏");
    assert!(
        commit.contains('？') || commit.contains('?'),
        "还原成问号，实际 {commit:?}"
    );
    assert!(router.current_frame().is_empty());
}

/// 只有一个 `?` 时退格照常删掉它。
#[test]
fn bare_question_can_be_deleted() {
    let mut router = router();
    router.press('?');
    router.press_key(keysym::BACKSPACE);
    assert!(router.current_frame().is_empty());
}

/// 表达式模式：`v1+2` 出算式结果，数字不当选候选的键。
#[test]
fn expression_mode_takes_digits_into_the_buffer() {
    let mut router = router();
    router.type_str("v1+2");
    assert!(
        router.page_texts().iter().any(|t| t == "3"),
        "该算出 3，实际 {:?}",
        router.page_texts()
    );
}
