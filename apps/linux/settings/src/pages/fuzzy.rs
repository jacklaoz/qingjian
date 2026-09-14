//! 「模糊音」页：九组勾选，缺省全关。
//!
//! 用勾选框不用开关：九项摆成三列，开关会挤（Windows 那边同样的理由）。

use std::rc::Rc;

use gtk::prelude::*;

use crate::form::{self, Row};
use crate::settings::Settings;

/// `(配置键, 显示名)`，配置键与 `FuzzyRules` 的字段名一致。
const RULES: &[(&str, &str)] = &[
    ("z_zh", "z = zh"),
    ("c_ch", "c = ch"),
    ("s_sh", "s = sh"),
    ("n_l", "n = l"),
    ("f_h", "f = h"),
    ("l_r", "l = r"),
    ("an_ang", "an = ang"),
    ("en_eng", "en = eng"),
    ("in_ing", "in = ing"),
];

pub fn build(settings: &Rc<Settings>) -> gtk::Box {
    let page = form::page();
    let fuzzy = settings.config().fuzzy;
    let current = [
        fuzzy.z_zh,
        fuzzy.c_ch,
        fuzzy.s_sh,
        fuzzy.n_l,
        fuzzy.f_h,
        fuzzy.l_r,
        fuzzy.an_ang,
        fuzzy.en_eng,
        fuzzy.in_ing,
    ];

    form::note(
        &page,
        "打开之后这两种写法互相都能打出来，代价是候选变多、命中的词按词频减半排序。缺省全关。",
    );
    let grid = gtk::Grid::new();
    grid.set_row_spacing(6);
    grid.set_column_spacing(24);
    grid.set_margin_top(12);
    for (index, ((key, title), on)) in RULES.iter().zip(current).enumerate() {
        let control = form::check(settings, Row::new(title, "fuzzy", key), on);
        grid.attach(&control, (index % 3) as i32, (index / 3) as i32, 1, 1);
    }
    page.append(&grid);
    page
}
