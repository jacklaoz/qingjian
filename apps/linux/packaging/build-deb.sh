#!/usr/bin/env bash
# 打四个 .deb：程序、设置界面、随包数据、本地整句模型。
#
# 为什么分开：数据加模型将近 150 MB，发行版不收这种体积的单包，而且数据与模型的更新节奏
# 跟代码不一样（重跑词库 / 重训模型不该逼用户重装程序）。程序包不依赖后两个——
# 缺语言模型退化成一元词频、缺模型不重排，Core 本来就支持（见 qingjian-platform 的 resources）。
# 设置界面单独一个包是因为它是**这一套里唯一要 GTK4 的东西**：输入法本体只链 libc，
# 合在一起会让 KDE / Qt 环境装个输入法就拖进整棵 GTK 树。
#
# 依赖声明没走 dh_shlibdeps，所以自己算：libc6 那条按符号版本（glibc_requirement），
# 动态库那几条按 NEEDED + dpkg -S（library_depends）。漏了它们不是装不上，而是装得上、跑不起来。
#
# 用法：apps/linux/packaging/build-deb.sh [--skip-data] [--skip-model]
# 产物在 target/deb/。
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

BIN_NAME=qingjian-linux
SETTINGS_BIN=qingjian-settings
OUT=target/deb
ARCH="$(dpkg --print-architecture)"
# 版本号写在 apps/linux/Cargo.toml 里（apps/* 各壳独立发布，见 docs/contributing.md）
VERSION="$(grep -m1 '^version = ' apps/linux/ime/Cargo.toml | cut -d'"' -f2)"
# Debian 的版本号不收 `-`：开发版 0.1.0-dev 落成 0.1.0~dev
DEB_VERSION="${VERSION//-/\~}"
MAINTAINER="Qingjian <https://qingjian.app>"

skip_data=0
skip_model=0
for arg in "$@"; do
    case "$arg" in
        --skip-data) skip_data=1 ;;
        --skip-model) skip_model=1 ;;
        *) echo "不认识的参数：$arg" >&2; exit 2 ;;
    esac
done

rm -rf "$OUT"
mkdir -p "$OUT"

# 一个二进制真正要求的 glibc 版本。
#
# 手写 control 没有 dh_shlibdeps 替我们算这条，缺了它的后果不是装不上而是**装得上、跑不起来**：
# dpkg 放行，启动时才报 `GLIBC_2.xx not found`。
# 只看强符号：Rust 标准库对 pidfd_spawnp / pidfd_getpid 这类是弱引用，老 glibc 上找不到会自己退回去。
glibc_requirement() {
    local found
    found="$(readelf --dyn-syms -W "$1" 2>/dev/null \
        | awk '$5 != "WEAK" && match($0, /GLIBC_[0-9.]+/) { print substr($0, RSTART + 6, RLENGTH - 6) }' \
        | sort -V | tail -1)"
    # readelf 不在或者符号表读不出来时别瞎猜，给一个这份代码肯定要的下限
    echo "${found:-2.34}"
}

# 二进制动态链接的库属于哪些包。
#
# 同样是 dh_shlibdeps 本该替我们做的事：readelf 取 NEEDED、ldd 找到实际路径、dpkg -S 问包名。
# 自动推导而不是手写，是因为**依赖会跟着代码变**：云联想接上之后 libssl / libcrypto 就进了 NEEDED，
# 手写的那份不会自己跟上，而漏了照样是「装得上、跑不起来」。
# libc / libm / libgcc / 动态链接器不在这里报，上面的 libc6 那条已经覆盖。
library_depends() {
    local bin="$1" soname path pkg
    local found=()
    while read -r soname; do
        case "$soname" in
            libc.so.*|libm.so.*|libgcc_s.so.*|ld-linux-*) continue ;;
        esac
        path="$(ldd "$bin" | awk -v want="$soname" '$1 == want { print $3 }')"
        [ -z "$path" ] && continue
        pkg="$(dpkg -S "$(readlink -f "$path")" 2>/dev/null | head -1 | cut -d: -f1)"
        [ -z "$pkg" ] && continue
        # Debian 的 time_t 转换给包名加了 t64 后缀（libssl3t64），转换之前的发行版上还叫老名字。
        # 写成「或」依赖，两边都装得上。
        case "$pkg" in
            *t64) found+=("$pkg | ${pkg%t64}") ;;
            *) found+=("$pkg") ;;
        esac
    done < <(LC_ALL=C readelf -d "$bin" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p')
    [ ${#found[@]} -eq 0 ] && return
    # `paste -sd', '` 是错的：-d 收的是**字符列表**，逗号与空格会轮流当分隔符，
    # 三项以上就会拼出 `a,b c` 这种 dpkg 不认的东西。一个字符连起来，再补空格。
    printf '%s\n' "${found[@]}" | awk '!seen[$0]++' | paste -sd, | sed 's/,/, /g'
}

# 写一个包的 control 文件。
# $1 包名 $2 依赖 $3 推荐 $4 一句话描述 $5 详细描述 $6 root 目录
write_control() {
    local name="$1" depends="$2" recommends="$3" summary="$4" detail="$5" root="$6"
    mkdir -p "$root/DEBIAN"
    {
        echo "Package: $name"
        echo "Version: $DEB_VERSION"
        echo "Section: utils"
        echo "Priority: optional"
        echo "Architecture: $ARCH"
        [ -n "$depends" ] && echo "Depends: $depends"
        [ -n "$recommends" ] && echo "Recommends: $recommends"
        echo "Maintainer: $MAINTAINER"
        echo "Homepage: https://qingjian.app"
        # 安装后占多少空间，按 KiB
        echo "Installed-Size: $(du -sk "$root" | cut -f1)"
        echo "Description: $summary"
        echo " $detail"
    } > "$root/DEBIAN/control"
}

build() {
    local name="$1" root="$2"
    fakeroot dpkg-deb --build --root-owner-group "$root" "$OUT/${name}_${DEB_VERSION}_${ARCH}.deb" > /dev/null
}

echo "==> 编译 release"
cargo build --release -p qingjian-linux -p qingjian-linux-settings

echo "==> qingjian（程序）"
ROOT="$OUT/qingjian"
mkdir -p "$ROOT/DEBIAN" "$ROOT/usr/bin" "$ROOT/usr/share/ibus/component" "$ROOT/usr/share/doc/qingjian"
install -m 755 "target/release/$BIN_NAME" "$ROOT/usr/bin/$BIN_NAME"
# 组件 XML 里的 exec 要写装好之后的绝对路径，不是构建机上的
"target/release/$BIN_NAME" --ibus-xml /usr/bin/$BIN_NAME > "$ROOT/usr/share/ibus/component/qingjian.xml"
install -m 644 LICENSE "$ROOT/usr/share/doc/qingjian/copyright"
# 装完 / 卸载后让 ibus 重扫组件，否则要手动 ibus restart 才看得到
cat > "$ROOT/DEBIAN/postinst" <<'HOOK'
#!/bin/sh
set -e
if command -v ibus >/dev/null 2>&1; then
    ibus write-cache --system >/dev/null 2>&1 || true
fi
echo "青简已装好。输入源列表里看不到它的话，先 'ibus restart' 或重新登录，"
echo "再到「设置 → 键盘 → 输入源」里把「青简」加进去。装好数据包之后跑 'qingjian-linux --check' 自检。"
HOOK
cat > "$ROOT/DEBIAN/postrm" <<'HOOK'
#!/bin/sh
set -e
if command -v ibus >/dev/null 2>&1; then
    ibus write-cache --system >/dev/null 2>&1 || true
fi
HOOK
chmod 755 "$ROOT/DEBIAN/postinst" "$ROOT/DEBIAN/postrm"
# 输入法本体不链 libibus（IBus 走的是 D-Bus，纯 Rust 的 zbus），但云联想经 reqwest 带进了 OpenSSL，
# 所以 NEEDED 里有 libssl / libcrypto——具体依赖哪个包由 library_depends 现查，不写死。
# ibus-gtk3 / ibus-gtk4 是 X11 会话里 GTK 应用接 ibus 要的（Wayland 走 text-input 协议，用不着）；
# Qt 的 ibus 插件随 libqt5gui5 / libqt6gui6 一起装，不用在这里点名。
# 没有中文字体的话候选窗（IBus 面板画的）是一排豆腐，所以也推荐上。
write_control qingjian \
    "ibus (>= 1.5), libc6 (>= $(glibc_requirement "target/release/$BIN_NAME")), $(library_depends "target/release/$BIN_NAME")" \
    "qingjian-data, qingjian-settings, ibus-gtk3, ibus-gtk4, fonts-noto-cjk" \
    "青简输入法" \
    "输入的不只是文字：候选旁附一条目标语言译词。词库与语言模型在 qingjian-data，本地整句模型在 qingjian-model，两者都可不装（缺了各少一个功能）。" \
    "$ROOT"
build qingjian "$ROOT"

echo "==> qingjian-settings（设置界面）"
ROOT="$OUT/qingjian-settings"
mkdir -p "$ROOT/DEBIAN" "$ROOT/usr/bin" "$ROOT/usr/share/applications" \
    "$ROOT/usr/share/pixmaps" "$ROOT/usr/share/doc/qingjian-settings"
install -m 755 "target/release/$SETTINGS_BIN" "$ROOT/usr/bin/$SETTINGS_BIN"
install -m 644 assets/icon/logo.png "$ROOT/usr/share/pixmaps/qingjian.png"
install -m 644 LICENSE "$ROOT/usr/share/doc/qingjian-settings/copyright"
# 设置界面进应用菜单
cat > "$ROOT/usr/share/applications/app.qingjian.Settings.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=青简设置
Name[en]=Qingjian Settings
Comment=设置青简输入法
Exec=/usr/bin/qingjian-settings
Icon=qingjian
Terminal=false
Categories=Settings;
Keywords=qingjian;input method;输入法;青简;
DESKTOP
write_control qingjian-settings \
    "qingjian (= $DEB_VERSION), libc6 (>= $(glibc_requirement "target/release/$SETTINGS_BIN")), $(library_depends "target/release/$SETTINGS_BIN")" \
    "" \
    "青简输入法的设置界面" \
    "左侧导航加各分节表单，读写 ~/.config/qingjian/config.toml，改完即时生效。单独一个包是因为它要 GTK4，而输入法本体不要。" \
    "$ROOT"
build qingjian-settings "$ROOT"

if [ "$skip_data" -eq 0 ]; then
    echo "==> qingjian-data（词库 / 语言模型 / 释义表）"
    ROOT="$OUT/qingjian-data"
    SHARE="$ROOT/usr/share/qingjian"
    mkdir -p "$SHARE/data/generated" "$SHARE/assets"
    if [ ! -f data/generated/dict.qj ]; then
        echo "没有 data/generated/dict.qj，先取产品数据（见 docs/notes/release.md）" >&2
        exit 1
    fi
    cp -r data/generated/. "$SHARE/data/generated/"
    cp -r assets/emoji assets/levels "$SHARE/assets/"
    write_control qingjian-data "" "" "青简的词库与语言模型" \
        "基础词库、整句转换的语言模型、中英 / 中日释义表、英文词表、emoji 表、词汇等级表与随包领域词库。没有它青简只能用几十条的样例词库。" \
        "$ROOT"
    build qingjian-data "$ROOT"
fi

if [ "$skip_model" -eq 0 ]; then
    echo "==> qingjian-model（本地整句模型）"
    ROOT="$OUT/qingjian-model"
    SHARE="$ROOT/usr/share/qingjian/data/model"
    mkdir -p "$SHARE"
    if [ ! -f data/model/model.qjm ]; then
        echo "没有 data/model/model.qjm，先取产品数据（见 docs/notes/release.md）" >&2
        exit 1
    fi
    install -m 644 data/model/model.qjm "$SHARE/model.qjm"
    write_control qingjian-model "" "" "青简的本地整句模型" \
        "字级 Transformer，敲完一段拼音停一下就重排整句候选，全程离线。不装就只用词库统计。" \
        "$ROOT"
    build qingjian-model "$ROOT"
fi

echo
echo "==> 成品"
ls -lh "$OUT"/*.deb | awk '{print "   " $9 "  " $5}'
echo
echo "装：sudo apt install $OUT/*.deb"
echo "    （用 apt 而不是 dpkg -i：Recommends 里的 ibus-gtk4 / 中文字体这些才会一起装上）"
