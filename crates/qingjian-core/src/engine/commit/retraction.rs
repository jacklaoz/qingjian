//! 撤销：上屏后被整个删掉的词，那次记的学习退回去。壳不在组句时按退格调 `Engine::note_backspace` 数删了几个字。

use super::LastCommit;
use crate::engine::Engine;
use crate::engine::input_log::{InputLogEntry, InputLogger};
use crate::engine::learning::Learner;

impl Engine {
    /// 最近删掉的上屏里有一次是同一段拼音（或它的前缀）、这次却选了别的词：把那次记的学习退回去，输入日志记一条 `retract`。
    /// 删掉几个词再从头重打是常事（「沃德 书」删掉重打成「我的 书」），所以往前找最近一次删干净的同段拼音；
    /// 重打后选的还是同一个词就不算选错，只把那条记录丢掉。
    /// 这次的拼音与删掉的哪次都对不上（`tui` 选了 腿、删掉改打 `push`）：删掉的词没留在文本里，学习一样退回去，
    /// 但不记 `retract`——那条事件说的是「同一段拼音该选这个」，换了拼音就不成立。
    pub(in crate::engine) fn apply_retraction(&mut self, input: &str, text: &str) {
        let Some(index) = self
            .recent_commits
            .iter()
            .rposition(|c| c.is_erased() && c.same_input(input))
        else {
            // 删掉的那次拼音与这次相近但不同（wode 删了重打 wodi）：重打了键，记进输入日志给敲错纠正当样本
            if let Some(erased) = self
                .recent_commits
                .iter()
                .rev()
                .find(|c| c.is_erased() && is_retyped(&c.input, input))
            {
                self.logger.record(InputLogEntry::Retype {
                    before: erased.input.clone(),
                    after: input.to_owned(),
                    of: erased.log_id,
                });
            }
            let erased: Vec<LastCommit> = self
                .recent_commits
                .iter()
                .filter(|c| c.is_erased() && c.text != text)
                .cloned()
                .collect();
            for commit in &erased {
                tracing::debug!(erased = %commit.text, chosen = %text, "删掉的词改打了别的，撤销它的学习");
                self.unlearn(commit);
            }
            self.recent_commits.retain(|c| !c.is_erased());
            return;
        };
        let last = self.recent_commits.remove(index);
        if last.text == text {
            return;
        }
        tracing::debug!(retracted = %last.text, chosen = %text, "上次选错了，撤销它的学习");
        self.logger.record(InputLogEntry::Retract {
            of: last.log_id,
            text: last.text.clone(),
            chosen: text.to_owned(),
        });
        self.unlearn(&last);
    }

    /// 退回一次上屏记过的全部学习：选择次数、输入串选择、词转移、敲错、整段合成词。
    fn unlearn(&mut self, commit: &LastCommit) {
        if let Some(chosen) = &commit.chosen {
            self.learner.unrecord(chosen);
        }
        if let Some(choice) = &commit.choice {
            self.learner
                .unrecord_choice(&commit.input, &choice.reading, &choice.text);
        }
        for transition in &commit.transitions {
            self.learner.unrecord_transition(
                transition.context(),
                &transition.word,
                transition.times,
            );
        }
        for (typed, intended) in &commit.typos {
            self.learner.unrecord_typo(typed, intended);
        }
        if let Some((key, choice)) = &commit.phrase {
            self.learner
                .unrecord_choice(key, &choice.reading, &choice.text);
        }
    }
}

/// 删掉重打的键串算不算「同一段拼音打错了」：都够长、不相等、编辑距离不超过 [`RETYPE_MAX_EDITS`]。
/// 差得更多的是在改别的内容，不算。
fn is_retyped(before: &str, after: &str) -> bool {
    before.len() >= RETYPE_MIN_LETTERS
        && after.len() >= RETYPE_MIN_LETTERS
        && before != after
        && edit_distance(before, after) <= RETYPE_MAX_EDITS
}

/// 跨上屏重打的键串最少几个字母才算数。
const RETYPE_MIN_LETTERS: usize = 3;

/// 跨上屏重打最多差几处编辑。
const RETYPE_MAX_EDITS: usize = 2;

/// Levenshtein 编辑距离（按字节，键串都是 ASCII）。
fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}
