use std::collections::HashMap;
use std::path::Path;

/// 一条符号记录：这个输入码对应的名字与符号们。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolEntry {
    /// 名字（`对勾`），挂在候选上当 annotation，与 emoji 候选的 `reading` 同一个位置。
    pub name: String,

    /// 符号们，按常用度排好。
    pub symbols: Vec<String>,
}

/// 输入码 → 符号。
///
/// **与 [`crate::EmojiTable`] 的区别在查法**：emoji 按**候选词的文本**查（笑 → 😄，所以它紧跟在词后面），
/// 符号按**用户敲的输入码**查。名字不在词库里的符号（`duigou` → ✔）只有后者查得到——
/// 这正是符号单独一张表的原因，不是数据分类上的洁癖。
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// 输入码 → 那一条。
    entries: HashMap<String, SymbolEntry>,
}

impl SymbolTable {
    /// 解析 TSV：`输入码\t名字\t符号 符号 …`，空行与 `#` 开头跳过；格式不对的行返回行号。
    ///
    /// 输入码是**全拼、不带声调**（`shenglvehao`）。同一个名字有两种拼法（`lve` / `lue`）时写成两行，
    /// 不在代码里做拼音变体推导：表是人工维护的，写清楚比猜准。
    pub fn parse(source: &str) -> Result<Self, usize> {
        let mut entries: HashMap<String, SymbolEntry> = HashMap::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut columns = line.split('\t');
            let (Some(code), Some(name), Some(symbols)) =
                (columns.next(), columns.next(), columns.next())
            else {
                return Err(index + 1);
            };
            if code.is_empty() || !code.chars().all(|c| c.is_ascii_lowercase()) {
                return Err(index + 1);
            }
            let entry = entries
                .entry(code.to_owned())
                .or_insert_with(|| SymbolEntry {
                    name: name.to_owned(),
                    symbols: Vec::new(),
                });
            for symbol in symbols.split(' ').filter(|s| !s.is_empty()) {
                if !entry.symbols.iter().any(|s| s == symbol) {
                    entry.symbols.push(symbol.to_owned());
                }
            }
        }
        Ok(Self { entries })
    }

    /// 从文件读一张表。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let source = std::fs::read_to_string(path)?;
        Self::parse(&source).map_err(|line| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("symbol table line {line}: malformed entry"),
            )
        })
    }

    /// 这个输入码对应的那一条；**只认完整相等**，不做前缀匹配。
    ///
    /// 前缀匹配会让符号在打到一半时就冒出来挤掉词候选（敲 `jia` 出 `＋`），
    /// 而用户绝大多数时候是在打字不是在找符号。
    pub fn lookup(&self, code: &str) -> Option<&SymbolEntry> {
        self.entries.get(code)
    }

    /// 并入另一张表（以后按语言分表时用）；同码的符号追加去重，名字以先来的为准。
    pub fn merge(&mut self, other: SymbolTable) {
        for (code, entry) in other.entries {
            let target = self.entries.entry(code).or_insert_with(|| SymbolEntry {
                name: entry.name.clone(),
                symbols: Vec::new(),
            });
            for symbol in entry.symbols {
                if !target.symbols.contains(&symbol) {
                    target.symbols.push(symbol);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_three_columns() {
        let table = SymbolTable::parse("duigou\t对勾\t✔ ✓ ☑\n").expect("解析");
        let entry = table.lookup("duigou").expect("查到");
        assert_eq!(entry.name, "对勾");
        assert_eq!(entry.symbols, ["✔", "✓", "☑"]);
    }

    /// 注释与空行跳过，同码两行合并且去重。
    #[test]
    fn comments_are_skipped_and_duplicates_merged() {
        let table =
            SymbolTable::parse("# 注释\n\njiantou\t箭头\t→ ←\njiantou\t箭头\t← ↑\n").expect("解析");
        assert_eq!(table.len(), 1);
        assert_eq!(
            table.lookup("jiantou").expect("查到").symbols,
            ["→", "←", "↑"]
        );
    }

    /// 只认完整相等：打到一半不该冒出符号。
    #[test]
    fn lookup_is_exact() {
        let table = SymbolTable::parse("jiantou\t箭头\t→\n").expect("解析");
        assert!(table.lookup("jian").is_none());
        assert!(table.lookup("jiantoux").is_none());
        assert!(table.lookup("jiantou").is_some());
    }

    /// 格式不对要说清楚是第几行：表是人工维护的，行号是唯一有用的线索。
    #[test]
    fn malformed_lines_report_their_number() {
        assert_eq!(
            SymbolTable::parse("duigou\t对勾\t✔\n少了一列\n").unwrap_err(),
            2
        );
        // 输入码只能是小写字母：大写或中文说明这一行的列写反了
        assert_eq!(SymbolTable::parse("对勾\tduigou\t✔\n").unwrap_err(), 1);
    }
}
