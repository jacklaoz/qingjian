//! `--check` 自检：把壳启动时要用的东西全找一遍并打印，装机后第一件事就是跑它。
//!
//! 只做定位与读配置，不装配 Engine（那是下一步的事），所以它在数据还没就位时也能跑完并说清楚缺什么。

use std::path::{Path, PathBuf};

use qingjian_platform::Config;

use crate::paths;

/// 随包数据：相对随包根的路径，加一句「缺了会怎样」。词库缺了就真不能用，其余都是少个功能。
const BUNDLED: [(&str, &str); 7] = [
    (
        "data/generated/dict.qj",
        "主词库，缺了只能用几十条的样例词库",
    ),
    ("data/generated/lm.qj", "语言模型，缺了整句退化成一元词频"),
    ("data/generated/english.tsv", "英文词表，缺了不出英文候选"),
    (
        "data/generated/glossary-en.qj",
        "中→英释义表，缺了候选旁没有英文译词",
    ),
    (
        "data/generated/glossary-zh.qj",
        "英→中释义表，缺了英文候选旁没有中文",
    ),
    ("assets/emoji/emoji-zh.tsv", "emoji 表，缺了不出 emoji 候选"),
    (
        "assets/levels",
        "词汇等级表目录，缺了统计不分 CEFR / JLPT 等级",
    ),
];

/// 跑一遍自检并打印。返回「要紧的东西齐不齐」：词库和用户目录任一缺失为 `false`。
pub fn run() -> bool {
    println!("青简 Linux 壳 {}\n", env!("CARGO_PKG_VERSION"));
    let user_ok = report_user_dirs();
    println!();
    let data_ok = report_bundled();
    println!();
    report_model();
    println!();
    report_config();
    let ok = user_ok && data_ok;
    println!(
        "\n自检结果：{}",
        if ok {
            "可以启动"
        } else {
            "缺少必需的东西，见上面标 ✗ 的行"
        }
    );
    ok
}

/// 三个 XDG 目录。建不出来就没法落学习数据，算要紧。
fn report_user_dirs() -> bool {
    println!("用户目录（XDG）");
    // 标签一律四个汉字：`{:<n}` 按字符数补空格，中文是双宽，长短不一就会错位
    let dirs = [
        ("配置目录", paths::config_dir()),
        ("数据目录", paths::data_dir()),
        ("日志目录", paths::state_dir()),
        ("词库目录", paths::dicts_dir()),
    ];
    let mut ok = true;
    for (name, result) in dirs {
        match result {
            Ok(dir) => println!("  ✓ {name}  {}", dir.display()),
            Err(error) => {
                println!("  ✗ {name}  {error}");
                ok = false;
            }
        }
    }
    ok
}

/// 随包数据。只有主词库算要紧，其余缺了各少一个功能。
fn report_bundled() -> bool {
    let Some(root) = qingjian_platform::resources::bundled_root() else {
        println!(
            "随包数据\n  ✗ 找不到随包根（可执行文件旁、仓库开发布局、/usr/share/qingjian 都没有）"
        );
        return false;
    };
    println!("随包数据（根：{}）", root.display());
    let mut dict_ok = false;
    for (rel, note) in BUNDLED {
        let path = root.join(rel);
        let found = path.exists();
        if rel == BUNDLED[0].0 {
            dict_ok = found;
        }
        let mark = if found { '✓' } else { '·' };
        println!(
            "  {mark} {rel:<34} {}",
            if found {
                size_note(&path)
            } else {
                note.to_owned()
            }
        );
    }
    if !dict_ok {
        println!("  ✗ 主词库不在，壳会回落 assets/sample/dict.tsv（几十条，只够冒烟测试）");
    }
    dict_ok
}

/// 本地整句模型：用户目录 `model/` 的优先，否则随包 `data/model/`（发行版包是 qingjian-model 那个）。
/// 缺了只是整句不重排，不算要紧，所以不进返回值。
fn report_model() {
    println!("本地整句模型");
    let root = qingjian_platform::resources::bundled_root().unwrap_or_else(|| PathBuf::from("."));
    match crate::dispatch::find_model(paths::data_dir().ok().as_deref(), &root) {
        Some(path) => println!("  ✓ {}  {}", path.display(), size_note(&path)),
        None => println!(
            "  · 没有（整句只用词库统计，不重排）；随包的该在 {}",
            root.join("data/model").display()
        ),
    }
}

/// 配置：不存在就写一份带注释的模板，然后读回来打印几个关键项。
fn report_config() {
    let path = match paths::config_file() {
        Ok(path) => path,
        Err(error) => {
            println!("配置\n  ✗ {error}");
            return;
        }
    };
    println!("配置（{}）", path.display());
    match Config::write_template_if_missing(&path) {
        Ok(true) => println!("  · 原来没有，已写出一份带注释的模板"),
        Ok(false) => {}
        Err(error) => println!("  ✗ 模板写不出来：{error}"),
    }
    match Config::load(&path) {
        Ok(config) => print_settings(&config),
        Err(error) => println!("  ✗ 读不了：{error}"),
    }
}

/// 打印几个一眼能看出配置有没有生效的项。
fn print_settings(config: &Config) {
    println!("  ✓ 学习语言      {}", config.general.learning_language);
    println!("  ✓ 每页候选      {}", config.general.page_size);
    println!(
        "  ✓ 云联想        {}",
        if config.predict.enabled {
            "开"
        } else {
            "关（缺省）"
        }
    );
    println!(
        "  ✓ 本地整句模型  {}",
        if config.model.enabled { "开" } else { "关" }
    );
    if config.apps.has_english_candidates_off() {
        println!("  · [apps] 名单在 Linux 上不会生效：Wayland 拿不到前台应用标识");
    }
}

/// 文件大小（目录不算），给一眼看出数据是不是完整的。
fn size_note(path: &Path) -> String {
    if path.is_dir() {
        return "目录在".to_owned();
    }
    match std::fs::metadata(path) {
        Ok(meta) => format!("{:.1} MB", meta.len() as f64 / (1024.0 * 1024.0)),
        Err(_) => "在".to_owned(),
    }
}
