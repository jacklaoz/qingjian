//! 云联想与释义兜底的接入。
//!
//! **这是第三份**：macOS 在 `host/config.rs`、Windows 在 `dispatch/reload/`，做的是同一件事——
//! 按 `[predict]` 建 [`CloudPredictor`] 与 [`CloudGlossFiller`] 挂到 Engine 上，关着或缺密钥就退回本地实现。
//! 收口方案见 `docs/plan/linux_plan.md` 的 L3。
//!
//! 请求本身不在这条线程上跑：`CloudPredictor` 自己起一条工人线程（内部一个 current-thread 运行时），
//! 所以它既不阻塞按键，也不占主循环那个多线程运行时。结果由 `Router::tick` 拉，
//! 拉到了这一帧就变了，主循环照 `ibus/active.rs` 记下的聚焦对象重画（见 `run.rs` 的 `present`）。

use qingjian_core::{Engine, NoGlossFiller, NoPredictor};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};

use super::Router;

impl Router {
    /// 启动时：按 `[predict]` 接上云联想与释义兜底。
    pub fn configure_cloud(&mut self, predict: &PredictConfig) {
        self.applied_predict = predict.clone();
        attach_cloud(&mut self.engine, predict);
    }

    /// 热加载：`[predict]` 变了才重建。正在等的联想由 `set_predictor` 作废，
    /// 云端给的整句补全也一并清掉——它是上一份配置的结果。
    pub fn apply_predict_config(&mut self, predict: &PredictConfig) {
        if *predict == self.applied_predict {
            return;
        }
        self.applied_predict = predict.clone();
        attach_cloud(&mut self.engine, predict);
        self.sentence = None;
    }
}

/// 按 `[predict]` 接云联想与释义兜底；关着或缺密钥就退回本地实现。
///
/// **失败一律只记日志**：没密钥、URL 不对、起不了线程都退回不联想，绝不因此影响打字——
/// 「输入优先于学习」那条（见 `docs/contributing.md`）。
fn attach_cloud(engine: &mut Engine, predict: &PredictConfig) {
    if !predict.enabled {
        tracing::info!("云联想未开启（[predict] enabled = false）");
        engine.set_predictor(Box::new(NoPredictor));
        engine.set_gloss_filler(Box::new(NoGlossFiller));
        return;
    }
    match CloudPredictor::new(predict) {
        Ok(predictor) => {
            engine.set_predictor(Box::new(predictor));
            tracing::info!(model = %predict.model, "云联想已接入");
        }
        Err(error) => {
            tracing::warn!(%error, "云联想接入失败（缺 API key？），退回本地候选");
            engine.set_predictor(Box::new(NoPredictor));
        }
    }
    // 释义兜底随云联想一起开：释义表里没有的词上屏后问一次云端，写进个人释义表
    match CloudGlossFiller::new(predict) {
        Ok(filler) => engine.set_gloss_filler(Box::new(filler)),
        Err(error) => {
            tracing::warn!(%error, "释义兜底未启用");
            engine.set_gloss_filler(Box::new(NoGlossFiller));
        }
    }
}
