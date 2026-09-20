//! 青简 IBus 前端。
//!
//! 与 Fcitx5 插件（`apps/linux/fcitx5`）平级的第二个前端：都只做「框架事件 → Server、帧 → 面板」，
//! 引擎在 `qingjian-linux-server` 那个进程里，两个前端可以同时连着同一个 Server
//! （Server 按连接代次 + 上下文隔离会话）。
//!
//! 现在到哪一步：socket 客户端与按键翻译已经通了，`--check` 能把一串键打进去看 Server 回的候选；
//! IBus 的 D-Bus 那一层（注册组件、`ProcessKeyEvent`、把帧交给 IBus 面板画）还没接，
//! 那一版的代码在 `26ece65`，坑记在 `docs/notes/linux-bringup.md`。

mod client;
mod error;
mod key;

use qingjian_platform::protocol::linux::{Capabilities, socket_path};

use client::Shared;
use error::IbusError;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--check") => {
            let typed = args.next().unwrap_or_else(|| "nihao".to_owned());
            if let Err(error) = check(&typed) {
                tracing::error!(%error, "自检没通过");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("用法：qingjian-linux-ibus --check [拼音]");
            eprintln!("  --check  连上 Server，把这串字母当按键打进去，打印回来的候选");
            eprintln!("           Server 要先起着：cargo run -p qingjian-linux-server");
            eprintln!("  IBus 的 D-Bus 接入还没做，见本文件的模块注释");
            std::process::exit(2);
        }
    }
}

/// 自检：连上 Server、开一个会话、把 `typed` 里的字母逐个当按键报上去，打印最后一帧的候选。
///
/// 这条路走通就说明前端这一侧的协议是对的——**IBus 还没接进来的时候，它是唯一能端到端验的东西**。
fn check(typed: &str) -> Result<(), IbusError> {
    let shared = Shared::default();
    tracing::info!(socket = %socket_path().display(), "连 Server");
    let frame = shared.with(|connection| {
        let session = connection.open_session("/qingjian/check")?;
        // 能力要排在按键前面，Server 在收到它之前不收按键
        connection.capabilities(session, Capabilities::default())?;
        // 真前端在输入框拿到焦点时报这一条，Server 据此把这个会话设成当前焦点
        connection.focus(session, true)?;
        let mut last = Default::default();
        for character in typed.chars() {
            let event = key::to_key_event(character as u32, 0);
            let reply = connection.key(session, event, false)?;
            tracing::debug!(key = %character, outcome = ?reply.outcome, "按键");
            if let Some(commit) = &reply.commit {
                tracing::info!(text = %commit, "上屏");
            }
            last = reply.frame;
        }
        connection.focus(session, false)?;
        connection.close_session(session)?;
        Ok(last)
    })?;

    tracing::info!(connected = shared.connected(), "自检走完");
    println!("打了「{typed}」，Server 回的一帧：");
    if frame.candidates.items.is_empty() {
        println!("  （没有候选）");
    }
    for (index, candidate) in frame.candidates.items.iter().enumerate() {
        println!("  {}. {}", index + 1, candidate.text);
    }
    Ok(())
}
