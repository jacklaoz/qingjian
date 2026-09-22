//! 符号候选：按**用户敲的输入码**查符号（`duigou` → ✔），插在候选前列。
//!
//! 与 emoji（[`crate::emoji`]）分开是因为查法不同：emoji 按候选词的文本查、紧跟在那个词后面，
//! 所以词库里没有的词就挂不上；符号按输入码查，名字不在词库里也打得出来。
//! 数据表人工维护（`assets/symbol/symbol-zh.tsv`），`输入码\t名字\t符号 符号 …`。

mod table;

pub use table::{SymbolEntry, SymbolTable};
