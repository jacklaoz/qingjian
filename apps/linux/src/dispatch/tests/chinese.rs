//! 中文输入：组句、选候选、翻页、光标、标点。

use super::*;

/// 敲拼音出候选，空格上屏第一个。
#[test]
fn typing_pinyin_gives_candidates_and_space_commits() {
    let mut router = router();
    let response = router.type_str("nihao");
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert!(response.commit.is_none(), "组句中不上屏");
    assert!(
        router.page_texts().contains(&"你好".to_owned()),
        "候选里有 你好，实际 {:?}",
        router.page_texts()
    );

    let response = router.press(' ');
    assert_eq!(response.commit.as_deref(), Some("你好"));
    assert!(router.current_frame().is_empty(), "上屏后收窗");
}

/// 数字键选当前页第 N 个。
#[test]
fn digit_selects_from_the_page() {
    let mut router = router();
    router.type_str("ni");
    let page = router.page_texts();
    assert!(page.len() >= 2, "至少两个候选，实际 {page:?}");
    let second = page[1].clone();
    let response = router.press('2');
    assert_eq!(response.commit.as_deref(), Some(second.as_str()));
}

/// 上下键挪高亮，选的就是高亮那个。
#[test]
fn arrows_move_the_highlight() {
    let mut router = router();
    router.type_str("ni");
    let page = router.page_texts();
    assert_eq!(router.current_frame().highlight, 0);
    router.press_key(keysym::DOWN);
    assert_eq!(router.current_frame().highlight, 1);
    router.press_key(keysym::UP);
    assert_eq!(router.current_frame().highlight, 0);
    let response = router.press(' ');
    assert_eq!(response.commit.as_deref(), Some(page[0].as_str()));
}

/// 缺省翻页键是 `[` `]`。
#[test]
fn bracket_keys_turn_pages() {
    let mut router = router_with(RouterConfig {
        page_size: 2,
        ..RouterConfig::default()
    });
    router.type_str("ni");
    let first_page = router.page_texts();
    assert!(
        router.current_frame().page_count > 1,
        "样例词库下 ni 不止一页"
    );
    let response = router.press(']');
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert_eq!(router.current_frame().page, 1);
    assert_ne!(router.page_texts(), first_page);
    router.press('[');
    assert_eq!(router.current_frame().page, 0);
}

/// 退格删一个字母，删光了收窗。
#[test]
fn backspace_deletes_one_letter() {
    let mut router = router();
    router.type_str("ni");
    assert_eq!(router.preedit(), "ni");
    router.press_key(keysym::BACKSPACE);
    assert_eq!(router.preedit(), "n");
    router.press_key(keysym::BACKSPACE);
    assert!(router.current_frame().is_empty());
}

/// Esc 清空缓冲、不上屏。
#[test]
fn escape_clears_without_committing() {
    let mut router = router();
    router.type_str("nihao");
    let response = router.press_key(keysym::ESCAPE);
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert!(response.commit.is_none());
    assert!(router.current_frame().is_empty());
}

/// 回车把拼音原样上屏。
#[test]
fn return_commits_the_raw_pinyin() {
    let mut router = router();
    router.type_str("nihao");
    let response = router.press_key(keysym::RETURN);
    assert_eq!(response.commit.as_deref(), Some("nihao"));
    assert!(router.current_frame().is_empty());
}

/// 左右键挪拼音光标，候选只按光标前那段算。
#[test]
fn cursor_scopes_the_candidates() {
    let mut router = router();
    router.type_str("nihao");
    for _ in 0..3 {
        router.press_key(keysym::LEFT);
    }
    assert!(
        router.page_texts().contains(&"你".to_owned()),
        "光标前只剩 ni，候选该有 你，实际 {:?}",
        router.page_texts()
    );
    router.press_key(keysym::END);
    assert!(router.page_texts().contains(&"你好".to_owned()));
}

/// 中文模式下的标点转全角。
#[test]
fn punctuation_becomes_full_width() {
    let mut router = router();
    let response = router.press(',');
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert_eq!(response.commit.as_deref(), Some("，"));
}

/// 关掉全角就原样交给应用。
#[test]
fn half_width_punctuation_passes_through() {
    let mut router = router_with(RouterConfig {
        full_width: false,
        ..RouterConfig::default()
    });
    let response = router.press(',');
    assert_eq!(response.outcome, KeyOutcome::Passthrough);
    assert!(response.commit.is_none());
}

/// 组句中敲半角标点进英文直输段：整段原样显示，不先上屏候选。
/// 这是 `docs/design/candidate-ui.md` 按键表定的行为（代价是没有了「拼音 + 标点 = 上屏候选 + 全角标点」，
/// 要先空格再打标点），三个平台一致。
#[test]
fn punctuation_while_composing_enters_a_raw_segment() {
    let mut router = router();
    router.type_str("dui");
    let response = router.press('\'');
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert!(response.commit.is_none(), "不上屏，进缓冲区");
    let response = router.press('m');
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert!(response.commit.is_none());
    assert!(
        router.preedit().contains('m'),
        "敲的还在 preedit 里，实际 {:?}",
        router.preedit()
    );
}

/// 英文直输段里回车整段原样上屏。
#[test]
fn raw_segment_commits_verbatim_on_return() {
    let mut router = router();
    router.type_str("no");
    router.press('-');
    router.type_str("way");
    let response = router.press_key(keysym::RETURN);
    assert_eq!(response.commit.as_deref(), Some("no-way"), "整段原样上屏");
}

/// 按住 Shift 打的大写字母：拼音先原样上屏，字母交还应用。
#[test]
fn shift_uppercase_flushes_pinyin_and_passes_through() {
    let mut router = router();
    router.type_str("ni");
    let response = router.press_with('A', SHIFT);
    assert_eq!(response.outcome, KeyOutcome::Consumed);
    assert_eq!(
        response.commit.as_deref(),
        Some("niA"),
        "拼音原样加上那个大写字母"
    );
}

/// 焦点离开：缓冲原样交出并清空。
#[test]
fn commit_raw_on_focus_out() {
    let mut router = router();
    router.type_str("nihao");
    assert_eq!(router.commit_raw().as_deref(), Some("nihao"));
    assert!(router.current_frame().is_empty());
    assert!(router.commit_raw().is_none(), "没在组句时没有东西交");
}

/// 切输入法 / 应用终止组句：丢掉不上屏。
#[test]
fn reset_drops_the_composition() {
    let mut router = router();
    router.type_str("nihao");
    router.reset_composition();
    assert!(router.current_frame().is_empty());
}
