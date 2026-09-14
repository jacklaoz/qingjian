//! Linux 壳的库部分：路径定位（[`paths`]）、按键翻译（[`keys`]）与自检（[`check`]），供 bin 与测试共用。
//!
//! 这里全是平台无关到「不认 IBus 也不认 Wayland」的那一层：`keys` 只认 X11 keysym，
//! 两条接入路线（IBus 的 `ProcessKeyEvent`、Wayland 的键盘抓取经 xkbcommon）翻出来的都是它。

pub mod check;
pub mod error;
pub mod keys;
pub mod paths;

pub use error::ShellError;
