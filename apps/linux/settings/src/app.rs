//! 设置窗口：左侧导航 + 右侧分节表单，与 Windows 设置同一个形状。

use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Orientation, ScrolledWindow, Stack, StackSidebar};

use crate::pages;
use crate::settings::Settings;

/// 窗口初始大小：够一页表单不横向挤，又不至于占满屏。
const SIZE: (i32, i32) = (860, 620);

pub fn build(app: &Application) {
    let settings = Rc::new(Settings::load());
    let stack = Stack::new();
    stack.set_hexpand(true);
    for (name, title, content) in pages::all(&settings) {
        // 每页各自滚动：模糊音那种短页不滚，云服务那种长页要滚
        let scroller = ScrolledWindow::new();
        scroller.set_child(Some(&content));
        scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        stack.add_titled(&scroller, Some(name), title);
    }

    let sidebar = StackSidebar::new();
    sidebar.set_stack(&stack);
    sidebar.set_size_request(160, -1);

    let layout = gtk::Box::new(Orientation::Horizontal, 0);
    layout.append(&sidebar);
    layout.append(&gtk::Separator::new(Orientation::Vertical));
    layout.append(&stack);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("青简设置")
        .default_width(SIZE.0)
        .default_height(SIZE.1)
        .child(&layout)
        .build();
    window.present();
}
