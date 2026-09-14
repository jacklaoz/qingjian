//! 英文模式：Caps Lock 直通大写、持久英文模式出候选、中英混输。

use super::*;

/// 持久英文模式：字母进缓冲区，出英文候选。
#[test]
fn english_mode_gives_word_candidates() {
    let mut router = router();
    router.set_english_mode(true);
    let input = |c: char| KeyInput::from_keysym(c as u32, 0).with_english_mode(true);
    for c in "hel".chars() {
        router.handle_key(input(c));
    }
    let page = router.page_texts();
    assert!(
        page.iter().any(|w| w == "hello"),
        "该有 hello 补全，实际 {page:?}"
    );
}

/// 英文模式下空格：没动过高亮就把敲的字母原样上屏，不选候选。
#[test]
fn space_commits_raw_unless_navigated() {
    let mut router = router();
    router.set_english_mode(true);
    let input =
        |c: char, state: u32| KeyInput::from_keysym(c as u32, state).with_english_mode(true);
    for c in "hel".chars() {
        router.handle_key(input(c, 0));
    }
    let response = router.handle_key(input(' ', 0));
    let commit = response.commit.expect("有上屏");
    assert!(commit.starts_with("hel"), "原样上屏，实际 {commit:?}");
}

/// 动过高亮之后的空格才选候选。
#[test]
fn space_takes_the_highlight_after_navigating() {
    let mut router = router();
    router.set_english_mode(true);
    let input = |c: char| KeyInput::from_keysym(c as u32, 0).with_english_mode(true);
    for c in "hel".chars() {
        router.handle_key(input(c));
    }
    router.handle_key(KeyInput::from_keysym(keysym::DOWN, 0).with_english_mode(true));
    let response = router.handle_key(input(' '));
    let commit = response.commit.expect("有上屏");
    assert_ne!(commit, "hel ", "选的是候选不是原串");
}

/// Caps Lock 亮着：无论中英模式都直接出大写英文，不组句。
#[test]
fn caps_lock_types_uppercase_directly() {
    let mut router = router();
    let response = router.handle_key(KeyInput::from_keysym('A' as u32, CAPS));
    assert_eq!(response.commit.as_deref(), Some("A"));
    assert!(router.current_frame().is_empty(), "不进组句");
}

/// 英文候选被配置关掉：英文模式下字母也不组句，直接出字母。
#[test]
fn english_candidates_can_be_turned_off() {
    let mut router = router_with(RouterConfig {
        english_candidates: false,
        ..RouterConfig::default()
    });
    router.set_english_mode(true);
    let response = router.handle_key(KeyInput::from_keysym('h' as u32, 0).with_english_mode(true));
    assert_eq!(response.commit.as_deref(), Some("h"));
    assert!(router.current_frame().is_empty());
}

/// 中文模式下整段字母像英文词时也出英文候选（中英混输）。
#[test]
fn chinese_mode_still_offers_english_words() {
    let mut router = router();
    router.type_str("hello");
    let page = router.page_texts();
    assert!(
        page.iter().any(|w| w == "hello"),
        "中英混输该给 hello，实际 {page:?}"
    );
}
