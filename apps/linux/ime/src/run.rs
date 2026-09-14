//! `--ibus` 模式：装好 Router，连上 IBus，一边服务一边看配置文件。

use std::time::{Duration, SystemTime};

use qingjian_platform::Config;

use crate::dispatch::RouterConfig;
use crate::error::ShellError;
use crate::ibus::service;
use crate::ibus::shared::Shared;
use crate::startup;

/// 配置文件轮询间隔与学习数据落盘的节拍，与 macOS 壳的定时器一致（那边是每秒看一次 mtime）。
const TICK: Duration = Duration::from_secs(1);

/// 起服务并一直跑，直到收到 Ctrl-C / SIGTERM。
pub async fn run() -> Result<(), ShellError> {
    let (router, config_path) = startup::build()?;
    let router = Shared::new(router);
    let connection = service::serve(router.clone())
        .await
        .map_err(|error| ShellError::Ibus(error.to_string()))?;
    tracing::info!("青简已接上 IBus，等按键");

    let mut watcher = ConfigWatcher::new(config_path);
    let mut ticker = tokio::time::interval(TICK);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                watcher.poll(&router);
                // 云联想 / 释义兜底的异步结果借这个节拍收，学习数据也在这里到点落盘
                router.guarded("定时节拍", |router| router.tick());
            }
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

/// 看着配置文件的修改时间，变了就重新读一份推给 Router。
/// 与 macOS 壳 `host/config_watch.rs` 同一个做法：不用 inotify，每秒看一次 mtime 足够且没有依赖。
struct ConfigWatcher {
    /// 配置文件路径。
    path: std::path::PathBuf,

    /// 上次看到的修改时间。
    seen: Option<SystemTime>,
}

impl ConfigWatcher {
    fn new(path: std::path::PathBuf) -> Self {
        let seen = modified(&path);
        Self { path, seen }
    }

    fn poll(&mut self, router: &Shared) {
        let current = modified(&self.path);
        if current == self.seen {
            return;
        }
        self.seen = current;
        match Config::load(&self.path) {
            Ok(config) => {
                tracing::info!(path = %self.path.display(), "配置变了，热加载");
                router.lock().set_config(RouterConfig::from(&config));
            }
            // 解析失败沿用上一份：用户正在编辑保存到一半也不能把输入法弄瘫
            Err(error) => tracing::error!(%error, "配置读不了，沿用上一份"),
        }
    }
}

fn modified(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}
