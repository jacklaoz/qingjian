//! 语言模型的数据文件。

use std::path::{Path, PathBuf};

use qingjian_lm::{BigramModel, LmError};

/// 语言模型的数据文件：`lm.qj` 优先，没有就用两张 TSV。
pub enum LanguageModelFiles {
    /// 打包过的 `.qj`，mmap 直接映射。
    Packed(PathBuf),

    /// 开发时直出的两张 TSV。
    Tsv {
        /// 一元词频表。
        unigram: PathBuf,

        /// 二元转移表。
        bigram: PathBuf,
    },
}

impl LanguageModelFiles {
    /// 在目录里找语言模型；两种都没有为 `None`（Engine 退化成一元词频整句）。
    pub fn find(dir: &Path) -> Option<Self> {
        let packed = dir.join("lm.qj");
        if packed.is_file() {
            return Some(Self::Packed(packed));
        }
        let unigram = dir.join("lm-unigram.tsv");
        let bigram = dir.join("lm-bigram.tsv");
        (unigram.is_file() && bigram.is_file()).then_some(Self::Tsv { unigram, bigram })
    }

    pub(super) fn load(&self) -> Result<BigramModel, LmError> {
        match self {
            Self::Packed(path) => BigramModel::from_path(path),
            Self::Tsv { unigram, bigram } => BigramModel::from_paths(unigram, bigram),
        }
    }
}
