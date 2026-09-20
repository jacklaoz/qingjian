//! 系统按键 → Core 的输入。
//!
//! 两条路进来的键最后都变成 [`KeyInput`]：方案 C 的 IBus `ProcessKeyEvent` 直接给 keysym 与修饰键掩码，
//! 方案 A 的 Wayland 键盘抓取给 keycode，由 xkbcommon 翻成同一套 keysym。所以这一层不认 IBus 也不认 Wayland。

mod function;
mod input;
pub mod keysym;
pub mod modifiers;

#[cfg(test)]
mod tests;

pub use function::FunctionKey;
pub use input::KeyInput;
