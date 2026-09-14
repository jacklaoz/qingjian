#!/usr/bin/env bash
# 打三个 .deb：程序、随包数据、本地整句模型。
#
# 为什么分三个：数据加模型将近 150 MB，发行版不收这种体积的单包，而且数据与模型的更新节奏
# 跟代码不一样（重跑词库 / 重训模型不该逼用户重装程序）。程序包不依赖后两个——
# 缺语言模型退化成一元词频、缺模型不重排，Core 本来就支持（见 qingjian-platform 的 resources）。
#
# 用法：apps/linux/packaging/build-deb.sh [--skip-data] [--skip-model]
# 产物在 target/deb/。
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

BIN_NAME=qingjian-linux
OUT=target/deb
ARCH="$(dpkg --print-architecture)"
# 版本号写在 apps/linux/Cargo.toml 里（apps/* 各壳独立发布，见 docs/contributing.md）
VERSION="$(grep -m1 '^version = ' apps/linux/Cargo.toml | cut -d'"' -f2)"
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

# 写一个包的 control 文件。
# $1 包名 $2 依赖 $3 一句话描述 $4 详细描述 $5 root 目录
write_control() {
    local name="$1" depends="$2" summary="$3" detail="$4" root="$5"
    mkdir -p "$root/DEBIAN"
    {
        echo "Package: $name"
        echo "Version: $DEB_VERSION"
        echo "Section: utils"
        echo "Priority: optional"
        echo "Architecture: $ARCH"
        [ -n "$depends" ] && echo "Depends: $depends"
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
cargo build --release -p qingjian-linux

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
HOOK
cat > "$ROOT/DEBIAN/postrm" <<'HOOK'
#!/bin/sh
set -e
if command -v ibus >/dev/null 2>&1; then
    ibus write-cache --system >/dev/null 2>&1 || true
fi
HOOK
chmod 755 "$ROOT/DEBIAN/postinst" "$ROOT/DEBIAN/postrm"
write_control qingjian "ibus (>= 1.5)" "青简输入法（Wayland）" \
    "输入的不只是文字：候选旁附一条目标语言译词。词库与语言模型在 qingjian-data，本地整句模型在 qingjian-model，两者都可不装。" \
    "$ROOT"
build qingjian "$ROOT"

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
    write_control qingjian-data "" "青简的词库与语言模型" \
        "基础词库、整句转换的语言模型、中英 / 中日释义表、英文词表、emoji 表、词汇等级表与随包领域词库。" \
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
    write_control qingjian-model "" "青简的本地整句模型" \
        "字级 Transformer，敲完一段拼音停一下就重排整句候选，全程离线。不装就只用词库统计。" \
        "$ROOT"
    build qingjian-model "$ROOT"
fi

echo
echo "==> 成品"
ls -lh "$OUT"/*.deb | awk '{print "   " $9 "  " $5}'
