//! 按键分派：把系统按键交给 Engine，产出「吃不吃这个键、上什么屏、画什么帧」。
//!
//! 这一层**不认 IBus 也不认 Wayland**：进来的是 [`KeyInput`]，出去的是 [`KeyResponse`]，
//! 所以两条接入路线共用它，也能脱离输入法框架整段测试（`tests/` 就是这么测的）。
//! 组句展示状态在 [`composed`]，按键规则在 [`key`]，配置项在 [`config`]，
//! 本地整句模型在 [`rescore`]，云联想在 [`cloud`]。
//!
//! 与 Windows Server 的 `dispatch` 相比少了会话分派：IBus 一个引擎实例服务当前焦点，
//! 没有「一个 Server 服务多个应用进程」那回事。

mod cloud;
mod composed;
mod config;
mod key;
mod rescore;
mod response;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use qingjian_core::Engine;
use qingjian_platform::LocalModelConfig;
use qingjian_platform::protocol::{Frame, KeyOutcome};
use qingjian_predict::PredictConfig;

use self::composed::Composed;
pub use self::config::RouterConfig;
use self::key::Effect;
pub use self::rescore::find_model;
use self::rescore::{ModelLoader, RescoreState};
pub use self::response::KeyResponse;
use crate::keys::KeyInput;

/// 学习数据落盘间隔（与 macOS / Windows 壳一致）。
const LEARNING_FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// 闲着时的节拍：配置文件轮询与学习数据落盘靠它，与 macOS 壳的定时器一致（那边也是每秒看一次 mtime）。
/// 等重排时节拍会短得多，见 [`Router::next_tick`]。
pub const IDLE_TICK: Duration = Duration::from_secs(1);

/// 按键 → Engine → 帧。一个 Engine 持当前组句。
pub struct Router {
    /// 输入内核，进程内唯一。
    engine: Engine,

    /// 每页候选数 / 云端槽位 / 翻页键等。
    config: RouterConfig,

    /// 当前组句的展示状态；没在组句时为 `None`。
    composed: Option<Composed>,

    /// 整句补全（preedit 右侧、Tab 上屏）；缓冲变化时清空。
    sentence: Option<String>,

    /// 删候选后的屏幕提示，随下一帧下发、下一次按键清。
    notice: Option<String>,

    /// 当前高亮候选在布局里的下标（跨页）。
    highlight: usize,

    /// 这轮查询里动过高亮：英文模式空格只在动过之后才选高亮词。
    navigated: bool,

    /// 上次把学习数据落盘的时间。
    last_flush: Instant,

    /// 本地整句模型（`.qjm` 或三件套目录）；没有模型文件为 `None`。
    model_path: Option<PathBuf>,

    /// 进行中的模型加载；加载完接到 Engine 上就清掉。
    model_loader: Option<ModelLoader>,

    /// 上次套用的 `[model]`，变了才重载 / 卸载。
    applied_model: LocalModelConfig,

    /// 重排的防抖 / 轮询进行态。
    rescore: RescoreState,

    /// 上次套用的 `[predict]`，变了才重建云联想。
    applied_predict: PredictConfig,
}

impl Router {
    pub fn new(engine: Engine, config: RouterConfig) -> Self {
        let mut router = Self {
            engine,
            config: RouterConfig {
                page_size: config.page_size.max(1),
                ..config
            },
            composed: None,
            sentence: None,
            notice: None,
            highlight: 0,
            navigated: false,
            last_flush: Instant::now(),
            model_path: None,
            model_loader: None,
            applied_model: LocalModelConfig::default(),
            rescore: RescoreState::default(),
            applied_predict: PredictConfig::default(),
        };
        router.apply_config();
        router
    }

    /// 处理一次按键。抬键一律放行，输入法只认按下。
    pub fn handle_key(&mut self, key: KeyInput) -> KeyResponse {
        self.notice = None;
        let (commit, outcome) = match self.apply_key(key) {
            Effect::Changed(commit) => {
                self.recompose();
                (commit, KeyOutcome::Consumed)
            }
            Effect::Navigated => (None, KeyOutcome::Consumed),
            Effect::Passthrough => (None, KeyOutcome::Passthrough),
        };
        let _ = self.poll_prediction();
        self.maybe_flush();
        KeyResponse {
            outcome,
            commit,
            frame: self.current_frame(),
        }
    }

    /// 定时节拍：接上加载好的模型、推进重排、拉一次云联想与释义兜底的异步结果。
    ///
    /// **只在这一帧真的变了时才回帧**：主循环据此决定要不要重画面板，
    /// 空转的节拍（每秒一次）不该往 D-Bus 上发信号。
    pub fn tick(&mut self) -> Option<Frame> {
        let learned = self.engine.poll_glosses();
        if learned > 0 {
            tracing::info!(learned, "释义兜底写入个人释义表");
        }
        self.attach_loaded_model();
        let mut changed = self.advance_rescoring();
        changed |= self.poll_prediction();
        self.maybe_flush();
        changed.then(|| self.current_frame())
    }

    /// 焦点离开 / 应用要求结束组句：缓冲原样交出并清空。没在组句时为 `None`。
    pub fn commit_raw(&mut self) -> Option<String> {
        if !self.composing() {
            return None;
        }
        let text = self.engine.take_raw();
        self.recompose();
        Some(text).filter(|text| !text.is_empty())
    }

    /// 丢掉组句不上屏（切输入法、应用强行终止组句）。
    pub fn reset_composition(&mut self) {
        if self.composing() {
            self.engine.clear();
        }
        self.engine.break_chain();
        self.recompose();
    }

    /// 持久中英模式（单击 Shift 切）。
    pub fn set_english_mode(&mut self, english: bool) {
        self.engine
            .set_english_mode(english && self.config.english_candidates);
    }

    /// 热加载：换一份配置并推给 Engine。
    pub fn set_config(&mut self, config: RouterConfig) {
        if self.config == config {
            return;
        }
        self.config = RouterConfig {
            page_size: config.page_size.max(1),
            ..config
        };
        self.apply_config();
        if self.composing() {
            self.recompose();
        }
    }

    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    /// 把配置里 Engine 关心的部分推过去。与 macOS `Host::apply_config` 的那条通路对应。
    fn apply_config(&mut self) {
        self.engine.set_shuangpin(self.config.shuangpin);
        self.engine.set_zhuyin_mode(self.config.zhuyin);
        self.engine.set_traditional(self.config.traditional);
        self.engine
            .set_full_width_punctuation(self.config.full_width);
    }

    pub fn flush_learning(&mut self) {
        self.engine.flush_learning();
        self.last_flush = Instant::now();
    }

    fn maybe_flush(&mut self) {
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.flush_learning();
        }
    }

    /// 给测试与自检用：看一眼内核。
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}
