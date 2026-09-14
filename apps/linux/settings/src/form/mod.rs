//! 表单控件的搭法。
//!
//! 各页只声明「哪一行、绑哪个配置键」（[`Row`]），控件怎么摆、改了怎么写回都收在这里，
//! 免得七个页各写一套间距和信号连接。控件一律用 GTK 原生的：按「显示面自绘、控件面原生」那条，
//! 候选窗才自绘，设置界面用系统控件（见 docs/contributing.md）。

mod row;

use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Align, CheckButton, DropDown, Entry, Label, Orientation, SpinButton, Switch};

pub use self::row::Row;
use crate::settings::Settings;

/// 一页的纵向容器。
pub fn page() -> gtk::Box {
    let page = gtk::Box::new(Orientation::Vertical, 6);
    page.set_margin_top(18);
    page.set_margin_bottom(18);
    page.set_margin_start(18);
    page.set_margin_end(18);
    page
}

/// 一个小标题，用来把一页里的行分组。
pub fn heading(page: &gtk::Box, text: &str) {
    let label = Label::new(Some(text));
    label.set_halign(Align::Start);
    label.set_margin_top(12);
    label.add_css_class("heading");
    page.append(&label);
}

/// 一段说明文字（比正文浅一号）。
pub fn note(page: &gtk::Box, text: &str) {
    let label = Label::new(Some(text));
    label.set_halign(Align::Start);
    label.set_wrap(true);
    label.set_xalign(0.0);
    label.add_css_class("dim-label");
    page.append(&label);
}

/// 一行：左边标题（底下可带一句小字），右边控件。
fn attach(page: &gtk::Box, spec: Row<'_>, control: &impl IsA<gtk::Widget>) {
    let line = gtk::Box::new(Orientation::Horizontal, 12);
    line.set_margin_top(6);
    let text = gtk::Box::new(Orientation::Vertical, 2);
    text.set_hexpand(true);
    let title = Label::new(Some(spec.title));
    title.set_halign(Align::Start);
    text.append(&title);
    if let Some(hint) = spec.hint {
        let hint = Label::new(Some(hint));
        hint.set_halign(Align::Start);
        hint.set_wrap(true);
        hint.set_xalign(0.0);
        hint.add_css_class("dim-label");
        hint.add_css_class("caption");
        text.append(&hint);
    }
    line.append(&text);
    let control = control.as_ref();
    control.set_valign(Align::Center);
    line.append(control);
    page.append(&line);
}

/// 开关行，绑一个布尔键。
pub fn switch(page: &gtk::Box, settings: &Rc<Settings>, spec: Row<'_>, initial: bool) {
    let control = Switch::new();
    control.set_active(initial);
    let (section, key) = (spec.section, spec.key);
    let settings = Rc::clone(settings);
    control.connect_active_notify(move |control| settings.set(section, key, control.is_active()));
    attach(page, spec, &control);
}

/// 下拉行，绑一个字符串键。`options` 是 `(配置里的值, 显示名)`。
pub fn choice(
    page: &gtk::Box,
    settings: &Rc<Settings>,
    spec: Row<'_>,
    options: &'static [(&'static str, &'static str)],
    current: &str,
) {
    let labels: Vec<&str> = options.iter().map(|(_, label)| *label).collect();
    let control = DropDown::from_strings(&labels);
    let selected = options
        .iter()
        .position(|(value, _)| *value == current)
        .unwrap_or(0);
    control.set_selected(selected as u32);
    let (section, key) = (spec.section, spec.key);
    let settings = Rc::clone(settings);
    control.connect_selected_notify(move |control| {
        if let Some((value, _)) = options.get(control.selected() as usize) {
            settings.set(section, key, *value);
        }
    });
    attach(page, spec, &control);
}

/// 数字行，绑一个整数键。
pub fn number(
    page: &gtk::Box,
    settings: &Rc<Settings>,
    spec: Row<'_>,
    range: (i64, i64),
    current: i64,
) {
    let control = SpinButton::with_range(range.0 as f64, range.1 as f64, 1.0);
    control.set_value(current as f64);
    let (section, key) = (spec.section, spec.key);
    let settings = Rc::clone(settings);
    control.connect_value_changed(move |control| {
        settings.set(section, key, control.value() as i64);
    });
    attach(page, spec, &control);
}

/// 文本行，绑一个字符串键。
///
/// **失焦或回车时才写**：每敲一个字母就写一次盘的话，输入法那边每秒看 mtime，
/// 会被半截的值反复热加载（接口地址敲到一半就是个坏地址）。
pub fn text(page: &gtk::Box, settings: &Rc<Settings>, spec: Row<'_>, current: &str) {
    let control = Entry::new();
    control.set_text(current);
    control.set_width_chars(24);
    let (section, key) = (spec.section, spec.key);
    let settings = Rc::clone(settings);
    let commit = move |control: &Entry| settings.set(section, key, control.text().as_str());
    let on_blur = commit.clone();
    control.connect_has_focus_notify(move |control| {
        if !control.has_focus() {
            on_blur(control);
        }
    });
    control.connect_activate(move |control| commit(control));
    attach(page, spec, &control);
}

/// 勾选框，绑一个布尔键。模糊音那种一排小项用它，比开关省地方。
pub fn check(settings: &Rc<Settings>, spec: Row<'_>, initial: bool) -> CheckButton {
    let control = CheckButton::with_label(spec.title);
    control.set_active(initial);
    let (section, key) = (spec.section, spec.key);
    let settings = Rc::clone(settings);
    control.connect_toggled(move |control| settings.set(section, key, control.is_active()));
    control
}

/// 一行按钮。
pub fn button(page: &gtk::Box, title: &str, on_click: impl Fn() + 'static) {
    let control = gtk::Button::with_label(title);
    control.set_halign(Align::Start);
    control.set_margin_top(6);
    control.connect_clicked(move |_| on_click());
    page.append(&control);
}
