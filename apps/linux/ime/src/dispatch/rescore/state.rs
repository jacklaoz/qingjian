//! 本地整句模型的两个节拍：停键后的防抖（到点才把整句路径送去后台打分）与结果轮询。
//! 主循环没有定时器，按 [`RescoreState::next_deadline`] 给的时长等下一次 `tick`；
//! 常数与 macOS 壳的 `RescoreMonitor`、Windows 的 `RescoreState` 相同。

use std::time::{Duration, Instant};

/// 停键多久才请求重排：比一般的击键间隔短，连着敲时不请求。
pub(super) const DEBOUNCE: Duration = Duration::from_millis(80);

/// 轮询间隔：模型一次几十毫秒。
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// 最长等多久；后台线程卡住时兜底。
pub(super) const MAX_WAIT: Duration = Duration::from_secs(2);

/// 重排的进行态。
#[derive(Debug, Default)]
pub(crate) struct RescoreState {
    /// 最近一次缓冲变化的时间；有它表示在等防抖到点。
    wanted_since: Option<Instant>,

    /// 本轮请求发出的时间；有它表示在等后台结果。
    polling_since: Option<Instant>,
}

impl RescoreState {
    /// 又敲了一键：重新计时。
    pub(super) fn schedule(&mut self) {
        self.wanted_since = Some(Instant::now());
    }

    /// 防抖到点了没。
    pub(super) fn debounce_elapsed(&self) -> bool {
        self.wanted_since
            .is_some_and(|since| since.elapsed() >= DEBOUNCE)
    }

    /// 请求已发出：开始等结果。
    pub(super) fn start_polling(&mut self) {
        self.wanted_since = None;
        self.polling_since = Some(Instant::now());
    }

    pub(super) fn polling(&self) -> bool {
        self.polling_since.is_some()
    }

    /// 等结果等太久了。
    pub(super) fn expired(&self) -> bool {
        self.polling_since
            .is_some_and(|since| since.elapsed() > MAX_WAIT)
    }

    /// 什么都不等。
    pub(super) fn stop(&mut self) {
        self.wanted_since = None;
        self.polling_since = None;
    }

    /// 下一次该来 `tick` 的时长；什么都不等为 `None`。
    pub(super) fn next_deadline(&self) -> Option<Duration> {
        if self.polling_since.is_some() {
            return Some(POLL_INTERVAL);
        }
        self.wanted_since
            .map(|since| DEBOUNCE.saturating_sub(since.elapsed()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 什么都不等时不给主循环任何期限，节拍就退回闲着的那一秒。
    #[test]
    fn idle_has_no_deadline() {
        let state = RescoreState::default();
        assert!(state.next_deadline().is_none());
        assert!(!state.polling());
    }

    /// 起了防抖：期限不超过防抖时长，而且还没到点。
    #[test]
    fn scheduling_waits_out_the_debounce() {
        let mut state = RescoreState::default();
        state.schedule();
        assert!(state.next_deadline().is_some_and(|wait| wait <= DEBOUNCE));
        assert!(!state.debounce_elapsed(), "刚起的防抖不该立刻到点");
    }

    /// 请求发出之后按轮询间隔来，防抖那一头清掉，免得同一批路径被送两次。
    #[test]
    fn polling_switches_to_the_poll_interval() {
        let mut state = RescoreState::default();
        state.schedule();
        state.start_polling();
        assert!(state.polling());
        assert!(!state.debounce_elapsed());
        assert_eq!(state.next_deadline(), Some(POLL_INTERVAL));
        assert!(!state.expired(), "刚发出的请求还没到兜底时限");
    }

    /// 停下就什么都不等了。
    #[test]
    fn stopping_clears_both() {
        let mut state = RescoreState::default();
        state.schedule();
        state.start_polling();
        state.stop();
        assert!(!state.polling());
        assert!(state.next_deadline().is_none());
    }
}
