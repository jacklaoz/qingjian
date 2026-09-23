# 青简的 RPM spec，出两个包：qingjian（Server、词库与本地整句模型、systemd 用户服务）与 qingjian-fcitx5（插件）。
#
# **装的是预编译好的文件**：rpm/assemble.sh 先把所有东西摆进 %%{qj_root}，这里只拷进 buildroot，所以没有 %%build。
# 从源码构建要把整棵 cargo 依赖树弄进来，自建分发不值得；真要进发行版官方仓库时再写 %%build 与 cargo 打包宏。
# 动态库依赖不手写：rpmbuild 扫 ELF 的 NEEDED 自动生成（libc.so.6(GLIBC_2.34)(64bit)、libFcitx5Core.so.7()(64bit) 这些）。
# 版本号由 build.sh 从 apps/linux/server/Cargo.toml 与 git 短哈希算好传进来；CHANGELOG 由维护者统一写，这里不带 %%changelog。

# 文件是外面编好、剥好的，别让 rpm 再剥符号 / 造 build-id 链接 / 出 debuginfo
%global __os_install_post %{nil}
%global _build_id_links none
%global debug_package %{nil}
# 没有 %%changelog（见上），别拿它定时间戳
%global source_date_epoch_from_changelog 0

Name:           qingjian
Version:        %{qj_version}
Release:        %{qj_release}%{?dist}
Summary:        青简输入法：Server 与词库
License:        GPL-3.0-or-later
URL:            https://qingjian.app
Recommends:     qingjian-fcitx5

%description
跨平台拼音输入法。本包是引擎（Server，systemd 用户服务）、随包词库与本地整句模型；
输入法前端在 qingjian-fcitx5。Server 装完就起。

%package fcitx5
Summary:        青简输入法的 fcitx5 插件
Requires:       %{name}%{?_isa} = %{version}-%{release}
Requires:       fcitx5
# Fedora 上没有它，登录时 fcitx5 不自己起、GNOME 仍走 IBus；别的发行版没有这个包时弱依赖直接忽略
Recommends:     fcitx5-autostart

%description fcitx5
让 fcitx5 能用青简；引擎在 qingjian 包里。装完重启 fcitx5，在配置工具里添加「青简」。

%install
cp -a %{qj_root}/. %{buildroot}/

# Server 挂到每个用户的会话里自启。--global 只对之后起来的用户 systemd 实例生效，而注销再登录得快时
# 实例根本不重起；所以已经登录的用户直接在他们正在跑的实例里重载并（重）启动（与 deb 的 postinst 一致）
%post
systemctl --global enable qingjian-server.service >/dev/null 2>&1 || :
for user in $(loginctl list-users --no-legend 2>/dev/null | awk '{print $2}'); do
  systemctl --user -M "$user@" daemon-reload >/dev/null 2>&1 || :
  systemctl --user -M "$user@" restart qingjian-server.service >/dev/null 2>&1 || :
done

%preun
if [ $1 -eq 0 ]; then
  systemctl --global disable qingjian-server.service >/dev/null 2>&1 || :
  for user in $(loginctl list-users --no-legend 2>/dev/null | awk '{print $2}'); do
    systemctl --user -M "$user@" stop qingjian-server.service >/dev/null 2>&1 || :
  done
fi

%files
%license /usr/share/licenses/qingjian/LICENSE
/usr/bin/qingjian-linux-server
/usr/share/qingjian
/usr/lib/systemd/user/qingjian-server.service
/usr/share/icons/hicolor/128x128/apps/qingjian.png

%files fcitx5
%{_libdir}/fcitx5/qingjian.so
/usr/share/fcitx5/addon/qingjian.conf
/usr/share/fcitx5/inputmethod/qingjian.conf
