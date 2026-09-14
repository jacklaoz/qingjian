//! 按键怎么作用到 Engine / 高亮上。分流规则与 macOS 壳的 `handle_text` / `handle_command`、
//! Windows 的 `apply_key` 对齐——三份手工对齐的孪生逻辑，收口方案见 `docs/plan/linux_plan.md` 的 L3。

use qingjian_core::{QUESTION_PREFIX, shortcut};

use super::{Effect, with_prefix};
use crate::dispatch::Router;
use crate::keys::{FunctionKey, KeyInput};

impl Router {
    /// 功能键靠 keysym，其余靠字符。组句中修饰键 + 数字是快捷键；带 Ctrl / Alt / Super 而没配到快捷键的键归应用。
    /// 表达式模式里 Shift + 数字打的是 `^ * ( )`，不当快捷键。
    pub(crate) fn apply_key(&mut self, key: KeyInput) -> Effect {
        if self.composing()
            && !self.engine.expression_mode()
            && let Some(digit) = key.digit_key()
            && let Some(effect) = self.apply_digit_shortcut(digit, key.modifiers.chord())
        {
            return effect;
        }
        if key.modifiers.has_command_key() {
            return Effect::Passthrough;
        }
        let Some(c) = key.character() else {
            return self.apply_function_key(key);
        };
        // Caps 亮着无论中英模式都直接出大写英文；英文候选只在持久英文模式、Caps 灭、配置允许时给。
        let caps = key.modifiers.caps;
        let english = caps || key.modifiers.english_mode;
        let english_candidates =
            key.modifiers.english_mode && !caps && self.config.english_candidates;
        // 缓冲区为空时敲 `?` 先进问字模式，中英文模式都行：后面跟字母就是在问字，跟别的键就还原成问号。
        if !self.composing() && c == QUESTION_PREFIX {
            self.engine.set_english_mode(false);
            self.engine.push(c);
            return Effect::Changed(None);
        }
        let question = self.composing() && self.engine.question_mode();
        // 英文模式下问字：Caps 让字母以大写送来，按小写收进问题。
        let c = if question && english && c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else {
            c
        };
        // 只有一个 `?` 时敲了字母以外的键：还原成问号上屏；空格只是「把这个 ? 上屏」，其他键按没在组句重新分派。
        if question && !c.is_ascii_lowercase() && self.engine.bare_question() {
            let mark = self.restore_bare_question(english);
            if c == ' ' {
                return Effect::Changed(Some(mark));
            }
            return with_prefix(Some(mark), self.apply_key(key), c);
        }
        // 英文组词中候选被关掉（Caps 亮 / 配置改了）：敲过的字母先原样上屏。
        let flushed = (self.composing() && !english_candidates && self.engine.english_mode())
            .then(|| self.engine.take_raw());
        self.engine
            .set_english_mode(english_candidates && !question);
        let effect = if english && !question {
            self.apply_english(c, english_candidates, key)
        } else {
            self.apply_chinese(c, key)
        };
        with_prefix(flushed, effect, c)
    }

    /// 缓冲区里只有一个 `?`：清掉，还原成问号（按当前模式的全角设置转）。
    fn restore_bare_question(&mut self, english: bool) -> String {
        self.engine.clear();
        if self.full_width_for(english)
            && let Some(mark) = self.engine.punctuate(QUESTION_PREFIX)
        {
            return mark.to_owned();
        }
        QUESTION_PREFIX.to_string()
    }

    /// 功能键。没在组句一律放行（回车顺带告诉 Core 这是段落边界）。
    fn apply_function_key(&mut self, key: KeyInput) -> Effect {
        let Some(function) = key.function() else {
            return Effect::Passthrough;
        };
        if !self.composing() {
            // 回车交给应用：文本流里是一个段落边界（macOS / Windows 壳同样记）
            if function == FunctionKey::Return {
                self.engine.note_passthrough('\n');
            }
            return Effect::Passthrough;
        }
        // 只有一个 `?` 时按了回车：回车就是「把这个 ? 上屏」，吞掉，否则聊天框会连消息一起发出去；
        // 退格 / Esc 照常删掉它。
        if self.engine.bare_question()
            && !matches!(function, FunctionKey::Backspace | FunctionKey::Escape)
        {
            let english = key.modifiers.caps || key.modifiers.english_mode;
            return Effect::Changed(Some(self.restore_bare_question(english)));
        }
        match function {
            FunctionKey::Backspace => {
                self.engine.backspace();
                Effect::Changed(None)
            }
            FunctionKey::Delete => {
                self.engine.delete_forward();
                Effect::Changed(None)
            }
            FunctionKey::Escape => {
                self.engine.clear();
                Effect::Changed(None)
            }
            FunctionKey::Return => Effect::Changed(Some(self.engine.take_raw())),
            FunctionKey::Tab if self.engine.english_mode() => {
                Effect::Changed(Some(self.commit_highlighted()))
            }
            // 中文模式 Tab：有整句补全就接受，否则交还应用（缩进 / 跳焦点）。
            FunctionKey::Tab => match self.sentence.take() {
                Some(sentence) => Effect::Changed(Some(self.engine.accept_prediction(&sentence))),
                None => Effect::Passthrough,
            },
            FunctionKey::Down => {
                self.move_highlight(1);
                Effect::Navigated
            }
            FunctionKey::Up => {
                self.move_highlight(-1);
                Effect::Navigated
            }
            FunctionKey::PageDown => {
                self.page(1);
                Effect::Navigated
            }
            FunctionKey::PageUp => {
                self.page(-1);
                Effect::Navigated
            }
            FunctionKey::Left => {
                self.engine.move_cursor_left();
                Effect::Changed(None)
            }
            FunctionKey::Right => {
                self.engine.move_cursor_right();
                Effect::Changed(None)
            }
            FunctionKey::Home => {
                self.engine.move_cursor_home();
                Effect::Changed(None)
            }
            FunctionKey::End => {
                self.engine.move_cursor_end();
                Effect::Changed(None)
            }
        }
    }

    /// 中文模式：小写字母进拼音；Shift 大写字母是临时打英文，组句中先把拼音原样上屏；
    /// 没在组句时的其他字符走全角标点（组句中的标点仍进英文直输段）。
    fn apply_chinese(&mut self, c: char, key: KeyInput) -> Effect {
        if c.is_ascii_uppercase() {
            let raw = self.composing().then(|| self.engine.take_raw());
            self.engine.note_passthrough(c);
            return with_prefix(raw, Effect::Passthrough, c);
        }
        if c.is_ascii_lowercase() {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        if !self.composing() {
            return self.apply_punctuation(c, key);
        }
        self.apply_printable(c, key)
    }

    /// 当前模式开着全角就让 Core 转（数字后的 `.` 保持半角）；转不了的原样交给应用并告知 Core。
    fn apply_punctuation(&mut self, c: char, key: KeyInput) -> Effect {
        let english = key.modifiers.caps || key.modifiers.english_mode;
        if self.full_width_for(english)
            && let Some(text) = self.engine.punctuate(c)
        {
            return Effect::Changed(Some(text.to_owned()));
        }
        self.engine.note_passthrough(c);
        Effect::Passthrough
    }

    /// 英文模式。开着候选：字母进缓冲区，空格 / 标点先把字母原样上屏（动过高亮的空格才选词）；
    /// 关着候选：字母由我们插入（大小写按 Shift）。其他键按英文模式那份全角设置转，转不了的交给应用。
    fn apply_english(&mut self, c: char, candidates: bool, key: KeyInput) -> Effect {
        let composing = self.composing();
        if !candidates {
            let raw = composing.then(|| self.engine.take_raw());
            let effect = if c.is_ascii_alphabetic() {
                self.engine.note_passthrough(c);
                Effect::Changed(Some(c.to_string()))
            } else {
                self.apply_punctuation(c, key)
            };
            return with_prefix(raw, effect, c);
        }
        if c.is_ascii_alphabetic()
            || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
        {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        let committed = composing.then(|| {
            if c == ' ' && self.navigated {
                self.commit_highlighted()
            } else {
                self.engine.take_raw()
            }
        });
        let effect = self.apply_punctuation(c, key);
        with_prefix(committed, effect, c)
    }

    /// 组句中的可打印键：数字选当前页第 N 个，翻页键翻页，空格上屏高亮，其余进英文直输段。
    /// 表达式模式（`v1+2`）里数字和运算符进算式；问字模式敲的还可能是码点（`u4e00`、`u+1f600`），数字与 `+` 进缓冲区；
    /// 微软 / 搜狗双拼的 `;` 是 ing 键，末尾有落单声母时进缓冲区。
    fn apply_printable(&mut self, c: char, key: KeyInput) -> Effect {
        let expression = self.engine.expression_mode();
        if (expression && shortcut::is_expression_char(c))
            || (self.engine.unicode_entry() && (c.is_ascii_digit() || c == '+'))
            || (c == ';' && self.engine.takes_semicolon())
        {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        if let Some(digit) = key.digit()
            && self.candidate_count() > 0
        {
            let page_size = self.config.page_size;
            let page = self.highlight / page_size;
            return Effect::Changed(self.commit_index(page * page_size + digit - 1));
        }
        if let Some(step) = page_key(c, self.config.page_keys) {
            self.page(step);
            return Effect::Navigated;
        }
        if c == ' ' {
            return Effect::Changed(Some(self.commit_highlighted()));
        }
        // 表达式 / 问字模式下的其他字符不进缓冲区（与 macOS / Windows 壳一致）：
        // 先把高亮候选上屏，再按没在组句处理这个键。
        if c != '\'' && (expression || self.engine.question_mode()) {
            let committed = self.commit_highlighted();
            let effect = self.apply_punctuation(c, key);
            return with_prefix(Some(committed), effect, c);
        }
        self.engine.push(c);
        Effect::Changed(None)
    }

    /// 上屏高亮候选；没有候选时缓冲原样上屏。
    fn commit_highlighted(&mut self) -> String {
        match self.commit_index(self.highlight) {
            Some(text) => text,
            None => self.engine.take_raw(),
        }
    }

    /// 当前模式的全角标点开关。
    fn full_width_for(&self, english: bool) -> bool {
        if english {
            self.config.english_full_width
        } else {
            self.config.full_width
        }
    }

    pub(crate) fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }
}

/// 翻页键对 `(上一页, 下一页)`：返回 -1 / +1。
fn page_key(c: char, page_keys: (char, char)) -> Option<isize> {
    if c == page_keys.0 {
        Some(-1)
    } else if c == page_keys.1 {
        Some(1)
    } else {
        None
    }
}
