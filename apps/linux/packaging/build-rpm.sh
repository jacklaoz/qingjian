#!/usr/bin/env bash
# 打四个 .rpm：程序、设置界面、数据、模型，分法与 build-deb.sh 一致。
#
# 本机（Ubuntu）没有 rpmbuild，所以**构建跑在 fedora 容器里**：宿主机先 cargo build --release，
# 把产物摆进 staging，容器里只做打包。这样也顺带有了验证环境——加 --verify 会在同一个容器里
# dnf install 装上再跑一次 --check，不至于交一个没跑过的 spec。
#
# 依赖不用手写：rpmbuild 自己扫 ELF 的 NEEDED 生成 Requires（libssl.so.3、GLIBC_2.34 这些）。
#
# 用法：apps/linux/packaging/build-rpm.sh [--skip-data] [--skip-model] [--verify]
# 产物在 target/rpm/RPMS/。
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

IMAGE="${QINGJIAN_RPM_IMAGE:-fedora:latest}"
OUT=target/rpm
STAGING="$OUT/staging"
SPEC=apps/linux/packaging/rpm/qingjian.spec
# 版本号写在 apps/linux/ime/Cargo.toml 里（apps/* 各壳独立发布，见 docs/contributing.md）
VERSION="$(grep -m1 '^version = ' apps/linux/ime/Cargo.toml | cut -d'"' -f2)"
# RPM 的 Version 不收 `-`，开发版按惯例落到 Release 上：0.1.0-dev → 0.1.0 / 0.dev
if [[ "$VERSION" == *-* ]]; then
    RPM_VERSION="${VERSION%%-*}"
    RPM_RELEASE="0.${VERSION#*-}"
else
    RPM_VERSION="$VERSION"
    RPM_RELEASE=1
fi

skip_data=0
skip_model=0
verify=0
for arg in "$@"; do
    case "$arg" in
        --skip-data) skip_data=1 ;;
        --skip-model) skip_model=1 ;;
        --verify) verify=1 ;;
        *) echo "不认识的参数：$arg" >&2; exit 2 ;;
    esac
done

if ! command -v docker >/dev/null 2>&1; then
    echo "没有 docker。本机装了 rpmbuild 的话可以自己跑 spec，否则装 docker（rpm 的构建与验证都在 fedora 容器里）" >&2
    exit 1
fi

echo "==> 编译 release"
cargo build --release -p qingjian-linux -p qingjian-linux-settings

echo "==> 摆 staging"
rm -rf "$OUT"
mkdir -p "$STAGING"/{bin,ibus,applications,icon,model}
install -m 755 target/release/qingjian-linux "$STAGING/bin/"
install -m 755 target/release/qingjian-settings "$STAGING/bin/"
install -m 644 LICENSE "$STAGING/LICENSE"
install -m 644 assets/icon/logo.png "$STAGING/icon/qingjian.png"
# 组件 XML 里的 exec 要写装好之后的绝对路径，不是构建机上的
target/release/qingjian-linux --ibus-xml /usr/bin/qingjian-linux > "$STAGING/ibus/qingjian.xml"
cat > "$STAGING/applications/app.qingjian.Settings.desktop" <<'DESKTOP'
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

with_data=0
with_model=0
if [ "$skip_data" -eq 0 ]; then
    if [ ! -f data/generated/dict.qj ]; then
        echo "没有 data/generated/dict.qj，先取产品数据（见 docs/notes/release.md），或者加 --skip-data" >&2
        exit 1
    fi
    mkdir -p "$STAGING/share/qingjian/data/generated" "$STAGING/share/qingjian/assets"
    cp -r data/generated/. "$STAGING/share/qingjian/data/generated/"
    cp -r assets/emoji assets/levels "$STAGING/share/qingjian/assets/"
    with_data=1
fi
if [ "$skip_model" -eq 0 ] && [ -f data/model/model.qjm ]; then
    install -m 644 data/model/model.qjm "$STAGING/model/model.qjm"
    with_model=1
fi

echo "==> 在 $IMAGE 容器里 rpmbuild"
docker run --rm -v "$PWD:/src" -w /src "$IMAGE" bash -euo pipefail -c "
    dnf install -y --setopt=install_weak_deps=False rpm-build >/dev/null
    rpmbuild -bb \
        --define '_topdir /src/$OUT' \
        --define 'qj_version $RPM_VERSION' \
        --define 'qj_release $RPM_RELEASE' \
        --define 'qj_staging /src/$STAGING' \
        --define 'qj_with_data $with_data' \
        --define 'qj_with_model $with_model' \
        /src/$SPEC
    chown -R $(id -u):$(id -g) /src/$OUT
"

if [ "$verify" -eq 1 ]; then
    echo "==> 在 $IMAGE 容器里装上再自检"
    docker run --rm -v "$PWD:/src" -w /src "$IMAGE" bash -euo pipefail -c "
        dnf install -y --setopt=install_weak_deps=False /src/$OUT/RPMS/*/*.rpm >/dev/null
        echo '--- 装出来的文件 ---'
        rpm -ql qingjian
        echo '--- 自动生成的依赖 ---'
        rpm -qR qingjian | grep -Ev '^(rpmlib|/bin/sh)' | sort
        echo '--- 自检 ---'
        qingjian-linux --check
    "
fi

echo
echo "==> 成品"
ls -lh "$OUT"/RPMS/*/*.rpm | awk '{print "   " $9 "  " $5}'
echo
echo "装：sudo dnf install $OUT/RPMS/*/*.rpm"
echo "    装完 ibus restart 或重新登录，再到「设置 → 键盘 → 输入源」里加「青简」。"
