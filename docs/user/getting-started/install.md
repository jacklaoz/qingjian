---
title: 安装
order: 1
description: macOS、Windows 与 Linux 的安装步骤：系统要求、安装包、首次打开被系统拦截时的处理、安装后的位置。
---

## macOS

**系统要求**：macOS 13 或更新。Apple Silicon 与 Intel 各有一个安装包；不确定机型时，点左上角  → 「关于本机」，查看「芯片」一行。

1. 下载 `Qingjian-<版本>-arm64.pkg`（Apple Silicon）或 `Qingjian-<版本>-x86_64.pkg`（Intel），双击安装，需要管理员密码。
2. 测试版尚无 Apple 开发者签名，首次打开会被系统拦截：到「系统设置 → 隐私与安全性」，在底部点「仍要打开」，再安装一次。
3. 安装完成后，输入法菜单中出现「青简」。

若未出现，到「系统设置 → 键盘 → 输入法 → 编辑 → +」，在「简体中文」下添加「青简」；仍未出现则注销后重新登录。

安装后：

- 菜单栏右上出现「中 / 英」状态项，点击可打开输入法菜单与偏好设置。
- 偏好设置在输入法菜单中，修改即时生效，没有「保存」按钮。
- 首次使用先到「偏好设置 → 通用」选择学习语言（英语或日语），候选旁的译词即为该语言。

## Windows

**系统要求**：64 位 Windows 10（1809 或更新）或 Windows 11。安装包自带设置界面所需的运行时，不必另装组件。

1. 下载 `Qingjian-<版本>-Setup.exe`，双击安装，需要管理员权限。
2. 测试版尚无正式的代码签名，SmartScreen 会拦截：点「更多信息 → 仍要运行」。
3. 安装完成后，青简出现在输入法列表中（任务栏右下的「中 / 英」或「拼」处，或按 `Win + Space` 切换），位于「中文(简体)」之下。

安装后：

- 任务栏右下显示当前为「中」或「英」；桌面上可另开一条悬浮状态条，见 [英文模式](../input/english-mode.md#悬浮状态条（Windows）)。
- 设置在开始菜单的「青简设置」，或悬浮状态条上的 ⚙。修改即时生效，没有「保存」按钮。
- 首次使用先到「设置 → 通用」选择学习语言（英语或日语）。

若某个程序中无法切换到青简，可注销后重新登录，或重启该程序。升级后，已打开的程序需重启才会使用新版本。

**已知问题**：测试版没有正式的代码签名，在任务栏搜索框、「设置」等系统应用中，候选窗口可能被应用遮挡而不可见；输入与上屏不受影响。此问题将在签名版中解决。

## Linux

**系统要求**：64 位 x86 的 Ubuntu 24.04、Debian 12 或更新的版本。桌面为 GNOME 或 KDE Plasma 均可，Wayland 会话与 X11 会话均可。青简通过输入法框架 IBus 工作，安装时会一并装上。

测试版提供四个安装包，作用如下：

| 安装包 | 内容 | 是否必需 |
| --- | --- | --- |
| `qingjian` | 输入法本体 | 必需 |
| `qingjian-settings` | 设置界面 | 建议安装 |
| `qingjian-data` | 词库、语言模型与译词表 | 建议安装，缺少时只有极少量候选 |
| `qingjian-model` | 本地整句模型 | 可选，见[本地整句模型](../input/local-model.md) |

1. 下载四个 `.deb` 文件，放在同一目录下。
2. 在该目录中执行：

   ```bash
   sudo apt install ./qingjian_*.deb ./qingjian-settings_*.deb ./qingjian-data_*.deb ./qingjian-model_*.deb
   ```

   需使用 `apt` 而非 `dpkg -i`：中文字体与 X11 会话所需的组件由 `apt` 一并装上。
3. 执行 `ibus restart`，或注销后重新登录。
4. 到「设置 → 键盘 → 输入源 → +」，在「汉语（中国）」下添加「青简」。

安装后：

- 用 `Super + Space` 在输入源之间切换（该组合键由桌面环境提供，可在系统设置中修改）。
- 设置界面在应用列表中名为「青简设置」，也可在终端执行 `qingjian-settings` 打开。修改即时生效，没有「保存」按钮。
- 首次使用先到「设置 → 通用」选择学习语言（英语或日语），候选旁的译词即为该语言。
- 候选窗口由 IBus 绘制，字体与配色跟随桌面环境的设置；「候选窗口」页的「外观」与「排布」两项在 Linux 上暂不生效。
- 云联想可用，但填密钥的方式与 macOS、Windows 不同（设置界面中没有密钥输入框），见 [云联想](../cloud/index.md#开启)。
  「翻译选中的文字」在 Linux 上不可用：Wayland 下读不到应用中选中的内容。

### 桌面环境与程序的差异

- **KDE Plasma**：部分发行版默认使用另一套输入法框架 Fcitx5。若输入源列表中找不到青简，到「系统设置 → 键盘 → 虚拟键盘」中选择 IBus，注销后重新登录。
- **X11 会话**：需安装 `ibus-gtk3`、`ibus-gtk4`（按上述命令安装时会一并装上），并执行 `im-config -n ibus` 后重新登录，否则部分程序中无法输入。Wayland 会话不需要这一步。
- **基于 Electron 的程序**（Visual Studio Code、Chrome 等）：在 Wayland 会话中需以 `--enable-wayland-ime` 参数启动，否则拼音不在光标处显示。
- **其他发行版**（Fedora、Arch、openSUSE 等）：使用下方的 Flatpak 包，或自行构建，步骤见项目仓库。

若安装后打不出字，在终端执行 `qingjian-linux --check`，它会逐项列出所需文件的位置与缺失情况。

### 其他发行版：Flatpak

Fedora、Arch、openSUSE 等没有 `.deb` 的发行版可用 Flatpak 包。**它比 `.deb` 多一个必需的步骤**：
青简是输入法，需要由系统的输入法框架启动，而框架只从系统目录中查找输入法，看不到 Flatpak 内部的内容，
因此装完后需手动注册一次。

1. 安装 Flatpak 包：

   ```bash
   flatpak install --user qingjian.flatpak
   ```

2. 注册到输入法框架（需要管理员密码，只需做一次；升级 Flatpak 包时不必重做）：

   ```bash
   flatpak run --command=qingjian-linux app.qingjian.Qingjian --ibus-xml \
       "flatpak run --command=qingjian-linux app.qingjian.Qingjian" \
       | sudo tee /usr/share/ibus/component/qingjian.xml > /dev/null
   ibus write-cache --system && ibus restart
   ```

3. 在「设置 → 键盘 → 输入源」中加入「青简」。

设置界面在应用列表中照常出现，也可执行 `flatpak run app.qingjian.Qingjian` 打开。
自检命令为 `flatpak run --command=qingjian-linux app.qingjian.Qingjian --check`。

**Flatpak 版的数据与 `.deb` 版不互通**：两者的设置与学习数据分别保存，位置见[数据与日志](../help/data-and-logs.md#本机文件)。
从一种装法换到另一种时，需自行复制这些文件，否则学到的词与统计不会跟过去。

## 升级与卸载

升级时直接安装新版安装包，学习数据与设置均保留。Linux 上重复执行上述 `apt install` 命令即可升级，`qingjian-data` 与 `qingjian-model` 未更新时可不重装。卸载见 [卸载](../help/uninstall.md)。
