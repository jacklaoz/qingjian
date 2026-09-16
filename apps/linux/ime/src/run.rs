//! `--ibus` 模式：装好 Router，连上 IBus，一边服务一边看配置文件。
//!
//! 节拍是**变速**的：闲着时一秒一次（配置文件的 mtime、学习数据落盘），
//! 等本地整句模型重排时按 [`Router::next_tick`](crate::dispatch::Router::next_tick) 给的
//! 防抖 80 ms / 轮询 20 ms 走。按键在 D-Bus 那条线上处理，处理完叫醒这里重算，
//! 否则停键之后要等满一秒才想起来去请求重排。

use std::time::{Instant, SystemTime};

use qingjian_platform::Config;
use qingjian_platform::protocol::Frame;
use zbus::object_server::SignalEmitter;

use crate::dispatch::{IDLE_TICK, RouterConfig};
use crate::error::ShellError;
use crate::ibus::active::ActiveEngine;
use crate::ibus::engine::present_frame;
use crate::ibus::service;
use crate::ibus::shared::Shared;
use crate::startup;

/// 起服务并一直跑，直到收到 Ctrl-C / SIGTERM。
pub async fn run() -> Result<(), ShellError> {
    let (router, config_path) = startup::build()?;
    let router = Shared::new(router);
    let active = ActiveEngine::default();
    let connection = service::serve(router.clone(), active.clone())
        .await
        .map_err(|error| ShellError::Ibus(error.to_string()))?;
    tracing::info!("青简已接上 IBus，等按键");

    let mut watcher = ConfigWatcher::new(config_path);
    loop {
        let wait = router.lock().next_tick();
        tokio::select! {
            _ = tokio::time::sleep(wait) => {
                watcher.poll(&router);
                // 本地模型重排、云联想与释义兜底的异步结果都借这个节拍收，学习数据也在这里到点落盘
                let frame = router.guarded("定时节拍", |router| router.tick()).flatten();
                if let Some(frame) = frame {
                    present(&connection, &active, &frame).await;
                }
            }
            // 按键处理完了：这一键多半又把重排的防抖推后了，重算节拍
            _ = router.woken() => {}
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("收到 Ctrl-C，落盘退出");
                break;
            }
        }
    }
    router.lock().flush_learning();
    drop(connection);
    Ok(())
}

/// 按键之外的重画（重排结果、云端词到了）：发给当前聚焦的那个引擎对象。
///
/// 按键路径上的发出者由 zbus 交进方法里，这里没有，所以照 [`ActiveEngine`] 记下的对象路径自己造一个。
/// 没有焦点（没人在打字）就不发——发到一个已经销毁的对象上只会在 ibus-daemon 那边留一条错误。
async fn present(connection: &zbus::Connection, active: &ActiveEngine, frame: &Frame) {
    let Some(path) = active.path() else {
        return;
    };
    let emitter = SignalEmitter::from_parts(connection.clone(), path.into());
    if let Err(error) = present_frame(&emitter, frame).await {
        tracing::warn!(%error, "按键之外重画候选窗失败");
    }
}

/// 看着配置文件的修改时间，变了就重新读一份推给 Router。
/// 与 macOS 壳 `host/config_watch.rs` 同一个做法：不用 inotify，每秒看一次 mtime 足够且没有依赖。
///
/// 节拍在重排期间会快到 20 ms 一次，所以这里自己按 [`IDLE_TICK`] 节流：配置文件不值得一秒 stat 五十遍。
struct ConfigWatcher {
    /// 配置文件路径。
    path: std::path::PathBuf,

    /// 上次看到的修改时间。
    seen: Option<SystemTime>,

    /// 上次去看的时间。
    checked: Instant,
}

impl ConfigWatcher {
    fn new(path: std::path::PathBuf) -> Self {
        let seen = modified(&path);
        Self {
            path,
            seen,
            checked: Instant::now(),
        }
    }

    fn poll(&mut self, router: &Shared) {
        if self.checked.elapsed() < IDLE_TICK {
            return;
        }
        self.checked = Instant::now();
        let current = modified(&self.path);
        if current == self.seen {
            return;
        }
        self.seen = current;
        match Config::load(&self.path) {
            Ok(config) => {
                tracing::info!(path = %self.path.display(), "配置变了，热加载");
                let mut router = router.lock();
                router.set_config(RouterConfig::from(&config));
                // `[model]` 不在 RouterConfig 里（它管的是加载 / 卸载模型，不是一条显示设置）
                router.apply_model_config(&config.model);
            }
            // 解析失败沿用上一份：用户正在编辑保存到一半也不能把输入法弄瘫
            Err(error) => tracing::error!(%error, "配置读不了，沿用上一份"),
        }
    }
}

fn modified(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}
