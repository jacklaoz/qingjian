//! 功能键：keysym 里那些不产生字符、但输入法要认的键。

use super::keysym;

/// 一个功能键。变体只收 Core 真正要分辨的那些，与 macOS 壳 `handle_command` / Windows
/// `apply_function_key` 认的是同一组；多出来的键一律不认，交还应用。
///
/// Shift + Tab 在 X11 是另一个 keysym（`ISO_Left_Tab`），这里仍归成 [`Tab`](Self::Tab)，
/// 由调用方看修饰键里的 shift 位区分——不给它单开一个变体，免得 Linux 的按键词汇跟另外两个平台对不上。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKey {
    /// 退格：删光标前一个字母。
    Backspace,

    /// Delete：删光标后一个字母。
    Delete,

    /// Tab：英文模式选词、中文模式接受整句补全，都没有就交还应用。
    Tab,

    /// 回车：把拼音原样上屏。
    Return,

    /// Esc：清空缓冲区。
    Escape,

    /// Home：光标移到开头。
    Home,

    /// End：光标移到末尾。
    End,

    /// 左方向键：光标左移一个字母。
    Left,

    /// 右方向键：光标右移一个字母。
    Right,

    /// 上方向键：高亮上移，到页边翻页。
    Up,

    /// 下方向键：高亮下移，到页边翻页。
    Down,

    /// PageUp：上一页候选。
    PageUp,

    /// PageDown：下一页候选。
    PageDown,
}

impl FunctionKey {
    /// keysym 认成功能键；不是功能键（或是输入法不认的那些）为 `None`。
    /// 主键盘区与小键盘区归到同一个变体：NumLock 灭着时小键盘的方向键走 `KP_*`。
    pub fn from_keysym(keysym: u32) -> Option<Self> {
        Some(match keysym {
            keysym::BACKSPACE => Self::Backspace,
            keysym::DELETE | keysym::KP_DELETE => Self::Delete,
            keysym::TAB | keysym::ISO_LEFT_TAB => Self::Tab,
            keysym::RETURN | keysym::KP_ENTER => Self::Return,
            keysym::ESCAPE => Self::Escape,
            keysym::HOME | keysym::KP_HOME => Self::Home,
            keysym::END | keysym::KP_END => Self::End,
            keysym::LEFT | keysym::KP_LEFT => Self::Left,
            keysym::RIGHT | keysym::KP_RIGHT => Self::Right,
            keysym::UP | keysym::KP_UP => Self::Up,
            keysym::DOWN | keysym::KP_DOWN => Self::Down,
            keysym::PAGE_UP | keysym::KP_PAGE_UP => Self::PageUp,
            keysym::PAGE_DOWN | keysym::KP_PAGE_DOWN => Self::PageDown,
            _ => return None,
        })
    }
}
