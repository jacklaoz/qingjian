//! 本地整句模型（与 macOS 壳的 `host/model.rs`、Windows 的 `dispatch/rescore/` 对齐）：
//! 后台加载、停键后请求重排、结果到了重画当前页。
//!
//! 按键回调里永远只跑词级模型；模型的意见在停键 80 毫秒后请求、几十毫秒后到，只换候选窗口里的整句候选，
//! 用户翻过页或动过高亮就不打扰。节拍由主循环驱动：[`Router::next_tick`] 说下次多久来一次
//! [`Router::tick`]（见 `run.rs`）。加载在 [`loader`]，进行态在 [`state`]。
//!
//! 前文暂时只有本会话历史：IBus 的 `SetSurroundingText` 要客户端报能力位，多数应用不报，还没接。

mod loader;
mod state;

use std::path::{Path, PathBuf};
use std::time::Duration;

use qingjian_core::CandidateLayout;
use qingjian_platform::LocalModelConfig;

use self::loader::Loaded;
pub(crate) use self::loader::ModelLoader;
pub(crate) use self::state::RescoreState;
use super::composed::Composed;
use super::{IDLE_TICK, Router};

/// 找模型（`.qjm` 单文件，或开发时的三件套目录）：用户目录 `model/` 优先（用户自己的模型），
/// 否则随包 `data/model/`（发行版包装在 `/usr/share/qingjian/data/model/`）；都没有为 `None`。
pub fn find_model(user_dir: Option<&Path>, bundled_root: &Path) -> Option<PathBuf> {
    let candidates = [
        user_dir.map(|dir| dir.join("model")),
        Some(bundled_root.join("data/model")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find_map(|dir| qingjian_neural::find_model(&dir))
}

impl Router {
    /// 启动时：记下模型文件，按 `[model] enabled` 决定要不要加载。
    pub fn configure_local_model(
        &mut self,
        model_path: Option<PathBuf>,
        config: &LocalModelConfig,
    ) {
        self.model_path = model_path;
        self.applied_model = config.clone();
        if config.enabled {
            self.load_local_model();
        }
    }

    /// 热加载：`[model]` 变了才重载 / 卸载。
    pub fn apply_model_config(&mut self, config: &LocalModelConfig) {
        if *config == self.applied_model {
            return;
        }
        self.applied_model = config.clone();
        if config.enabled {
            self.load_local_model();
        } else {
            self.unload_local_model();
        }
    }

    /// 在后台线程加载模型；没有模型文件就什么都不做。
    fn load_local_model(&mut self) {
        if self.model_loader.is_some() || self.engine.has_sentence_scorer() {
            return;
        }
        let Some(path) = &self.model_path else {
            tracing::info!("没有本地整句模型文件，不重排");
            return;
        };
        self.model_loader = ModelLoader::spawn(path);
    }

    /// 卸掉模型（配置关掉）。
    fn unload_local_model(&mut self) {
        self.model_loader = None;
        self.engine.set_async_sentence_scorer(None);
        self.rescore.stop();
        tracing::info!("本地整句模型已卸载（[model] enabled = false）");
    }

    /// 加载线程有结果了就接到 Engine 上；每次按键 / 节拍顺手看一眼，不阻塞。
    pub(super) fn attach_loaded_model(&mut self) {
        let Some(loader) = &self.model_loader else {
            return;
        };
        match loader.poll() {
            Loaded::Pending => {}
            Loaded::Done(result) => {
                match *result {
                    Ok(scorer) => {
                        self.engine
                            .set_async_sentence_scorer(Some(Box::new(scorer)));
                        // 模型上线了：日志里补一条会话信息，之后的条目知道重排开着
                        self.engine.log_session(env!("CARGO_PKG_VERSION"), "linux");
                    }
                    Err(error) => tracing::warn!(%error, "本地整句模型加载失败，不重排"),
                }
                self.model_loader = None;
            }
            Loaded::Gone => self.model_loader = None,
        }
    }

    /// 主循环下次该多久后来一次 [`Router::tick`]：在等重排就按它的节拍（防抖 80 ms / 轮询 20 ms），
    /// 否则就是闲着的一秒。
    pub fn next_tick(&self) -> Duration {
        self.rescore
            .next_deadline()
            .map_or(IDLE_TICK, |deadline| deadline.min(IDLE_TICK))
    }

    /// 组句结束：什么都不等了。
    pub(super) fn stop_rescoring(&mut self) {
        self.rescore.stop();
    }

    /// 缓冲变化之后：有整句路径等着打分就起防抖计时，否则停下。
    pub(super) fn schedule_rescoring(&mut self) {
        if self.engine.rescoring_pending() {
            self.rescore.schedule();
        } else {
            self.rescore.stop();
        }
    }

    /// 防抖到点就发请求；在等结果就收一次。收到了重查并重建候选布局，回 `true` 让调用方重画。
    pub(super) fn advance_rescoring(&mut self) -> bool {
        if self.engine.composition().is_empty() {
            self.rescore.stop();
            return false;
        }
        if self.rescore.debounce_elapsed() {
            if self.engine.request_rescoring() {
                self.rescore.start_polling();
            } else {
                self.rescore.stop();
            }
        }
        if !self.rescore.polling() {
            return false;
        }
        if !self.engine.poll_rescoring() {
            // 等太久多半是前文变了（上屏后接着打下一段）、结果作废；真卡住也只是这轮不重排
            if self.rescore.expired() {
                tracing::debug!("等本地整句模型超时，本轮不重排");
                self.rescore.stop();
            }
            return false;
        }
        self.rescore.stop();
        if self.highlight >= self.config.page_size || self.navigated {
            return false;
        }
        self.requery_rescored()
    }

    /// 分回来了：按重排后的顺序重建候选布局，云端词留着。回 `true` 表示这一帧变了。
    fn requery_rescored(&mut self) -> bool {
        let Ok(query) = self.engine.query() else {
            return false;
        };
        let page_size = self.config.page_size;
        let cloud_slots = self.config.cloud_slots;
        let Some(Composed::Candidates {
            preedit,
            cursor,
            layout,
        }) = self.composed.as_mut()
        else {
            return false;
        };
        let cloud = layout.cloud().to_vec();
        let mut rebuilt =
            CandidateLayout::new(query.candidates.items.clone(), page_size, cloud_slots);
        if !cloud.is_empty() {
            rebuilt.set_cloud(cloud);
        }
        *layout = rebuilt;
        *preedit = query.marked_segments().iter().map(Into::into).collect();
        *cursor = query.marked_cursor();
        // 真机上重排没效果时，这条是唯一能看出「模型算完了、帧也重建了」的地方
        tracing::debug!("本地整句模型重排完，重画当前页");
        // 这次查询又记下了一批要打分的（前文在结果回来之前换过）：再来一轮
        self.schedule_rescoring();
        true
    }
}
