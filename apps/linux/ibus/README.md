# 青简 IBus 前端

与 [`../fcitx5`](../fcitx5) 平级的**第二个前端**：同一个 Server、同一套协议，换一个输入法框架。
引擎在 [`../server`](../server) 那个进程里，前端只做两件事——把框架的按键与焦点事件转成协议事件发过去，
把 Server 回的帧交给框架的面板画。

两个前端可以同时装着、同时连着一个 Server：Server 按「连接代次 + 上下文」隔离会话，互不干扰。
用户实际用哪个由 `$XMODIFIERS` 决定（GNOME 上内建的是 IBus）。

## 现在到哪一步

| 部分 | 状态 |
| --- | --- |
| socket 客户端（连接、握手、会话、事件） | 通了，有测试 |
| 按键翻译（keysym + 修饰键 → `KeyEvent`） | 通了，有测试 |
| `--check` 端到端自检 | 通了 |
| IBus 的 D-Bus 层（注册组件、`ProcessKeyEvent`、面板） | **还没接** |

D-Bus 那一层的代码在提交 `26ece65`（走进程内引擎的那一版，`apps/linux/ime/src/ibus/`），
接进来时要把它对 `Router` 的调用换成这里的 [`client::Connection`](src/client/connection.rs)；
真机上的坑记在 [`docs/notes/linux-bringup.md`](../../../docs/notes/linux-bringup.md)。

## 自检

Server 要先起着（它会 bind `$XDG_RUNTIME_DIR/qingjian.sock`，`QINGJIAN_SOCKET` 可以换路径）：

```bash
cargo run -p qingjian-linux-server          # 一个终端
cargo run -p qingjian-linux-ibus -- --check nihao   # 另一个终端
```

打印 Server 回的候选就说明前端这一侧的协议是对的。打 `"nihao "`（带空格）能看到上屏那一条。

## 改动注意

[`src/key/mapping.rs`](src/key/mapping.rs) 的 keysym → 虚拟键码表**必须与 Fcitx5 插件的
`../fcitx5/src/key/mapping.cpp` 一字不差**：两个前端喂同一个 Server、同一套 `[shortcut]` 配置，
映射不一致的话同一个键在两个框架下行为就不一样。改一边要改另一边。
