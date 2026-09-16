# 青简的 RPM spec，一份出四个包：程序 / 设置界面 / 数据 / 模型，与 .deb 的分法一致。
#
# **装的是预编译好的二进制**（`build-rpm.sh` 先在宿主机 cargo build --release，再把产物摆进
# %%{qj_staging}），所以没有 %%build。理由与 Flatpak 那边一样：从源码构建要把整棵 cargo 依赖树
# 弄进来，自建分发不值得；真要进 Fedora 官方仓库时再写 %%build 与 cargo 打包宏。
#
# 依赖不用手写：rpmbuild 会自己扫 ELF 的 NEEDED 生成 Requires（libssl.so.3()(64bit)、
# libc.so.6(GLIBC_2.34)(64bit) 这些），这是 rpm 比手写 control 的 .deb 省事的地方。
#
# 日期不从 %%changelog 取（这份 spec 是脚本生成版本号的，CHANGELOG 由维护者统一写，见 contributing.md）。

# **已知的一处偏严**：pidfd_spawnp / pidfd_getpid 是 Rust 标准库的弱引用（glibc 里没有就退回
# fork+exec），实际门槛是 GLIBC_2.34，但 rpm 的自动依赖不分强弱，仍会列出 GLIBC_2.39。
# 试过 %%global __requires_exclude（覆盖与追加两种写法），在 Fedora 44 的依赖生成器上都不生效，
# 没有再往下挖——代价只是把 RHEL 9 与已 EOL 的 Fedora 39 挡在门外，Fedora 40+ / openSUSE
# Tumbleweed 都是 2.39 以上。deb 那边是自己算的，所以 .deb 的门槛是 2.34，两边差一档，知道就好。

# 二进制是外面编好的，别让 rpm 去剥符号 / 造 build-id 链接
%global __os_install_post %{nil}
%global _build_id_links none
%global debug_package %{nil}

Name:           qingjian
Version:        %{qj_version}
Release:        %{qj_release}%{?dist}
Summary:        青简输入法
License:        GPL-3.0-or-later
URL:            https://qingjian.app
# 输入法本体只要 ibus；libssl 之类由自动依赖补上
Requires:       ibus >= 1.5
# 缺了数据只有几十条样例词库，症状是「装好了打不出字」，所以强烈建议一起装
Recommends:     %{name}-data = %{version}-%{release}
Recommends:     %{name}-settings = %{version}-%{release}
# X11 会话里 GTK 应用接 ibus 要这两个（Wayland 走 text-input 协议，用不着）
Recommends:     ibus-gtk3
Recommends:     ibus-gtk4
# 候选窗是 IBus 面板画的，没有中文字体就是一排豆腐
Recommends:     google-noto-sans-cjk-fonts

%description
输入的不只是文字：候选旁附一条目标语言译词。
拼音转换、词库、排序、学习与翻译全部在本机完成。
词库与语言模型在 qingjian-data，本地整句模型在 qingjian-model，两者都可不装（缺了各少一个功能）。

%package settings
Summary:        青简输入法的设置界面
Requires:       %{name} = %{version}-%{release}

%description settings
左侧导航加各分节表单，读写 ~/.config/qingjian/config.toml，改完即时生效。
单独一个包是因为它要 GTK4，而输入法本体不要。

%package data
Summary:        青简的词库与语言模型
BuildArch:      noarch

%description data
基础词库、整句转换的语言模型、中英 / 中日释义表、英文词表、emoji 表、词汇等级表与随包领域词库。
没有它青简只能用几十条的样例词库。

%package model
Summary:        青简的本地整句模型
BuildArch:      noarch

%description model
字级 Transformer，敲完一段拼音停一下就重排整句候选，全程离线。不装就只用词库统计。

%prep
# 什么都不做：源码不进包，二进制由 build-rpm.sh 摆好

%build
# 同上

%install
rm -rf %{buildroot}
install -Dm755 %{qj_staging}/bin/qingjian-linux %{buildroot}%{_bindir}/qingjian-linux
install -Dm755 %{qj_staging}/bin/qingjian-settings %{buildroot}%{_bindir}/qingjian-settings
install -Dm644 %{qj_staging}/ibus/qingjian.xml %{buildroot}%{_datadir}/ibus/component/qingjian.xml
install -Dm644 %{qj_staging}/applications/app.qingjian.Settings.desktop \
    %{buildroot}%{_datadir}/applications/app.qingjian.Settings.desktop
install -Dm644 %{qj_staging}/icon/qingjian.png %{buildroot}%{_datadir}/pixmaps/qingjian.png
install -Dm644 %{qj_staging}/LICENSE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE
%if 0%{?qj_with_data}
mkdir -p %{buildroot}%{_datadir}/qingjian
cp -r %{qj_staging}/share/qingjian/. %{buildroot}%{_datadir}/qingjian/
%endif
%if 0%{?qj_with_model}
mkdir -p %{buildroot}%{_datadir}/qingjian/data/model
install -Dm644 %{qj_staging}/model/model.qjm %{buildroot}%{_datadir}/qingjian/data/model/model.qjm
%endif

# 装完 / 卸载后让 ibus 重扫组件，否则要手动 ibus restart 才看得到
%post
if command -v ibus >/dev/null 2>&1; then
    ibus write-cache --system >/dev/null 2>&1 || true
fi

%postun
if command -v ibus >/dev/null 2>&1; then
    ibus write-cache --system >/dev/null 2>&1 || true
fi

%files
%license %{_datadir}/licenses/%{name}/LICENSE
%{_bindir}/qingjian-linux
%{_datadir}/ibus/component/qingjian.xml

%files settings
%{_bindir}/qingjian-settings
%{_datadir}/applications/app.qingjian.Settings.desktop
%{_datadir}/pixmaps/qingjian.png

%if 0%{?qj_with_data}
%files data
%{_datadir}/qingjian/data/generated
%{_datadir}/qingjian/assets
%endif

%if 0%{?qj_with_model}
%files model
%{_datadir}/qingjian/data/model
%endif

%changelog
* Wed Sep 16 2026 Qingjian <https://qingjian.app>
- 版本号由 build-rpm.sh 从 apps/linux/ime/Cargo.toml 读入；变更记录见仓库 CHANGELOG
