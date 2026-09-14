//! 一次按键：壳收到的原始形态。

use qingjian_platform::protocol::KeyModifiers;

use super::{FunctionKey, keysym, modifiers};

/// 一次按下的键。**不复用协议里的 `KeyEvent`**：那个的 `virtual_key` 是 Windows 的 VK 码，
/// Linux 这边是 X11 keysym，两者的取值空间不一样，混用迟早出错。
///
/// 字符与功能键都是从 `keysym` 算出来的，不另存一份，免得两者对不上。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyInput {
    /// X11 keysym。IBus 直接给，Wayland 那条路由 xkbcommon 从 keycode 翻出来。
    pub keysym: u32,

    /// 按下时的修饰键状态，外加 Caps Lock 与输入法自己的中英模式。
    pub modifiers: KeyModifiers,
}

impl KeyInput {
    /// 从 keysym 与修饰键掩码建一次按键。`english_mode` 是输入法状态、不在掩码里，缺省 `false`，
    /// 用 [`with_english_mode`](Self::with_english_mode) 补。
    pub fn new(keysym: u32, state: u32) -> Self {
        Self {
            keysym,
            modifiers: modifiers::from_mask(state),
        }
    }

    /// 补上持久的中英模式位。
    pub fn with_english_mode(mut self, english: bool) -> Self {
        self.modifiers.english_mode = english;
        self
    }

    /// 这个键敲出来的可见字符；功能键与纯修饰键为 `None`。
    pub fn character(self) -> Option<char> {
        keysym::to_char(self.keysym)
    }

    /// 这个键是不是输入法要认的功能键。
    pub fn function(self) -> Option<FunctionKey> {
        FunctionKey::from_keysym(self.keysym)
    }

    /// 敲出来是数字 1–9 的键（选候选用）。按字符认，所以 Shift 出的 `!@#` 不算；
    /// 小键盘数字也算（`KP_1`–`KP_9` 在 [`keysym::to_char`] 里已经折成 `'1'`–`'9'`）。
    pub fn digit(self) -> Option<usize> {
        let c = self.character()?;
        ('1'..='9').contains(&c).then(|| c as usize - '0' as usize)
    }
}
