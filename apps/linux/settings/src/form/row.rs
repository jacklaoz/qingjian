//! 一行表单绑的是哪个配置键。

/// 一行的描述：显示什么、写回哪里。
///
/// 收成一个结构体而不是一串参数：控件种类有五六种，每种都要 `(标题, 小字, 分节, 键)` 四样，
/// 摊平了签名就长到看不出哪个是哪个。
#[derive(Clone, Copy)]
pub struct Row<'a> {
    /// 左边的标题。
    pub title: &'a str,

    /// 标题底下的一句小字；不需要就 `None`。
    pub hint: Option<&'a str>,

    /// 写回 `config.toml` 的哪个分节。
    pub section: &'static str,

    /// 写回哪个键。
    pub key: &'static str,
}

impl<'a> Row<'a> {
    /// 不带小字的一行。
    pub fn new(title: &'a str, section: &'static str, key: &'static str) -> Self {
        Self {
            title,
            hint: None,
            section,
            key,
        }
    }

    /// 补一句小字。
    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }
}
