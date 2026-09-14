//! 按键翻译的测试：keysym 取值钉死在这里，改了表这些会先红。

use super::*;

/// Shift 掩码（X11 位 0）。
const SHIFT: u32 = 1 << 0;

/// Caps Lock 掩码（X11 位 1）。
const LOCK: u32 = 1 << 1;

/// Control 掩码（X11 位 2）。
const CONTROL: u32 = 1 << 2;

/// Mod1 = Alt 掩码（X11 位 3）。
const MOD1: u32 = 1 << 3;

/// 抬键标志位。
const RELEASE: u32 = 1 << 30;

#[test]
fn ascii_keysym_is_its_own_code_point() {
    assert_eq!(keysym::to_char(0x61), Some('a'));
    assert_eq!(keysym::to_char(0x41), Some('A'));
    assert_eq!(keysym::to_char(0x20), Some(' '));
    assert_eq!(keysym::to_char(0x27), Some('\''));
}

#[test]
fn latin1_high_range_is_a_code_point_too() {
    assert_eq!(keysym::to_char(0xe9), Some('é'));
    assert_eq!(keysym::to_char(0xa0), Some('\u{a0}'));
}

/// 0x7f–0x9f 是 Latin-1 的控制字符段，不当可见字符。
#[test]
fn latin1_control_range_has_no_character() {
    assert_eq!(keysym::to_char(0x7f), None);
    assert_eq!(keysym::to_char(0x90), None);
}

#[test]
fn unicode_keysym_strips_the_prefix() {
    assert_eq!(keysym::to_char(0x0100_4e2d), Some('中'));
    assert_eq!(keysym::to_char(0x0101_f600), Some('😀'), "星文平面也要认");
    assert_eq!(
        keysym::to_char(0x0100_0009),
        None,
        "Unicode 段里的控制字符不算"
    );
    assert_eq!(
        keysym::to_char(0x0120_0000),
        None,
        "超出 Unicode 上限的不认"
    );
}

#[test]
fn function_keys_have_no_character() {
    for key in [
        keysym::BACKSPACE,
        keysym::TAB,
        keysym::RETURN,
        keysym::ESCAPE,
        keysym::LEFT,
        keysym::DELETE,
    ] {
        assert_eq!(keysym::to_char(key), None, "keysym {key:#x} 不该有字符");
    }
}

#[test]
fn keypad_digits_read_as_digits() {
    assert_eq!(keysym::to_char(keysym::KP_0), Some('0'));
    assert_eq!(keysym::to_char(keysym::KP_9), Some('9'));
    assert_eq!(KeyInput::new(keysym::KP_0 + 3, 0).digit(), Some(3));
}

/// Shift + Tab 在 X11 是另一个 keysym，仍要认成 Tab，由修饰键区分方向。
#[test]
fn iso_left_tab_is_still_tab() {
    assert_eq!(
        FunctionKey::from_keysym(keysym::ISO_LEFT_TAB),
        Some(FunctionKey::Tab)
    );
    let key = KeyInput::new(keysym::ISO_LEFT_TAB, SHIFT);
    assert_eq!(key.function(), Some(FunctionKey::Tab));
    assert!(key.modifiers.shift);
}

/// NumLock 灭着时小键盘的方向键是另一段 keysym，归到同一个变体。
#[test]
fn keypad_navigation_maps_to_the_same_keys() {
    assert_eq!(
        FunctionKey::from_keysym(keysym::KP_LEFT),
        Some(FunctionKey::Left)
    );
    assert_eq!(
        FunctionKey::from_keysym(keysym::KP_PAGE_DOWN),
        Some(FunctionKey::PageDown)
    );
    assert_eq!(
        FunctionKey::from_keysym(keysym::KP_ENTER),
        Some(FunctionKey::Return)
    );
}

#[test]
fn letters_are_not_function_keys() {
    assert_eq!(FunctionKey::from_keysym(0x61), None);
    assert_eq!(FunctionKey::from_keysym(0x0100_4e2d), None);
}

#[test]
fn modifier_mask_maps_to_the_shared_struct() {
    let key = KeyInput::new(0x61, CONTROL | SHIFT | MOD1 | LOCK);
    assert!(key.modifiers.ctrl);
    assert!(key.modifiers.shift);
    assert!(key.modifiers.alt, "Mod1 是 Alt");
    assert!(key.modifiers.caps, "Lock 是 Caps Lock");
    assert!(!key.modifiers.win);
    assert!(
        !key.modifiers.english_mode,
        "中英模式是输入法状态，不在掩码里"
    );
}

#[test]
fn english_mode_is_added_on_top() {
    let key = KeyInput::new(0x61, 0).with_english_mode(true);
    assert!(key.modifiers.english_mode);
    assert!(!key.modifiers.caps, "两者是两个位，别混");
}

#[test]
fn release_events_are_told_apart() {
    assert!(modifiers::is_release(RELEASE));
    assert!(modifiers::is_release(RELEASE | SHIFT));
    assert!(!modifiers::is_release(SHIFT));
}

/// Shift 出来的是符号不是数字，不能当选候选的数字键。
#[test]
fn shifted_digits_are_not_candidate_keys() {
    assert_eq!(KeyInput::new('1' as u32, 0).digit(), Some(1));
    assert_eq!(KeyInput::new('!' as u32, SHIFT).digit(), None);
    assert_eq!(KeyInput::new('0' as u32, 0).digit(), None, "0 不选候选");
}
