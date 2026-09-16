//! IBus 接入（方案 C）：把 Router 的帧交给 IBus 自带的面板画。
//!
//! GNOME 与 KDE 的 Wayland 会话只有这条路——客户端不能给自己的窗口定位，自绘候选窗摆不到光标处
//! （见 `docs/plan/linux_plan.md`）。所以这不是过渡方案，是那两个桌面上的终态。
//!
//! 分两层：[`variant`] 是 IBus 那套 GVariant 对象的序列化，[`view`] 把 Router 的
//! [`Frame`](qingjian_platform::protocol::Frame) 折成「preedit 什么样、候选表什么样、辅助行写什么」。
//! D-Bus 服务在 [`engine`] 与 [`factory`]；按键之外的重画（本地模型重排、云联想）要知道
//! 当前聚焦的是哪个引擎对象，记在 [`active`]。

pub mod active;
pub mod component;
pub mod engine;
pub mod factory;
pub mod service;
pub mod shared;
pub mod variant;
pub mod view;
