#!/usr/bin/env bash
# 打一个 Flatpak：输入法引擎 + 设置界面 + 随包数据，装在 app.qingjian.Qingjian 这个 id 下。
#
# **先读这一段再用。** 输入法走 Flatpak 有一处绕不过去的手工步骤：
# ibus-daemon 只从 /usr/share/ibus/component/（与 IBUS_COMPONENT_PATH）找引擎，
# **不看 XDG_DATA_DIRS**，而 Flatpak 导出文件正是靠 XDG_DATA_DIRS（2026-09-16 实测：
# 放进 XDG_DATA_HOME 与 XDG_DATA_DIRS 的组件 XML 都不被发现）。
# 所以装完之后还要用 sudo 往宿主机放一个组件 XML，脚本会把它生成好并打印命令。
# 因此 Flatpak 在这里**不是 .deb 的替代**，是给非 Debian 系发行版的一条路。
#
# 用法：apps/linux/packaging/build-flatpak.sh [--skip-data] [--skip-model] [--bundle] [--install]
#   --bundle   额外导出单文件 target/flatpak/qingjian.flatpak（拿给别人 flatpak install 用）
#   --install  构建完直接装到本用户（--user）
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

APP_ID=app.qingjian.Qingjian
# 清单里的 sources 路径是**相对清单所在目录**解析的，所以构建时把它拷到产物目录，
# 让 `path: staging` 正好指着 $OUT/staging，不必在清单里写一串 ../../..
MANIFEST_SRC=apps/linux/packaging/flatpak/$APP_ID.yml
OUT=target/flatpak
STAGING="$OUT/staging"
REPO="$OUT/repo"
# runtime 版本可用环境变量顶掉（flathub 上现有 49 / 50）
RUNTIME_VERSION="${QINGJIAN_FLATPAK_RUNTIME:-49}"

skip_data=0
skip_model=0
bundle=0
install=0
for arg in "$@"; do
    case "$arg" in
        --skip-data) skip_data=1 ;;
        --skip-model) skip_model=1 ;;
        --bundle) bundle=1 ;;
        --install) install=1 ;;
        *) echo "不认识的参数：$arg" >&2; exit 2 ;;
    esac
done

# flatpak-builder 可以是本机命令，也可以是装成 flatpak 的 org.flatpak.Builder
builder() {
    if command -v flatpak-builder >/dev/null 2>&1; then
        flatpak-builder "$@"
    else
        flatpak run org.flatpak.Builder "$@"
    fi
}

have_builder() {
    command -v flatpak-builder >/dev/null 2>&1 || flatpak info org.flatpak.Builder >/dev/null 2>&1
}

if ! command -v flatpak >/dev/null 2>&1; then
    echo "没有 flatpak，先装它（Ubuntu：sudo apt install flatpak）" >&2
    exit 1
fi
if ! have_builder; then
    cat >&2 <<'HINT'
没有 flatpak-builder。装一个（不需要 root）：
    flatpak install --user flathub org.flatpak.Builder
HINT
    exit 1
fi
if ! flatpak info "org.gnome.Platform//$RUNTIME_VERSION" >/dev/null 2>&1; then
    cat >&2 <<HINT
没有 org.gnome.Platform//$RUNTIME_VERSION（约 1 GB，含 GTK4 与 OpenSSL）。装它：
    flatpak install --user flathub org.gnome.Platform//$RUNTIME_VERSION org.gnome.Sdk//$RUNTIME_VERSION
换个版本：QINGJIAN_FLATPAK_RUNTIME=50 $0
HINT
    exit 1
fi

echo "==> 编译 release"
cargo build --release -p qingjian-linux -p qingjian-linux-settings

echo "==> 摆 staging"
rm -rf "$OUT"
mkdir -p "$STAGING/bin" "$STAGING/share/applications" "$STAGING/share/metainfo" \
    "$STAGING/share/icons/hicolor/512x512/apps"
install -m 755 target/release/qingjian-linux "$STAGING/bin/"
install -m 755 target/release/qingjian-settings "$STAGING/bin/"
install -m 644 "apps/linux/packaging/flatpak/$APP_ID.desktop" "$STAGING/share/applications/"
install -m 644 "apps/linux/packaging/flatpak/$APP_ID.metainfo.xml" "$STAGING/share/metainfo/"
# 图标名必须等于 app-id，否则桌面环境认不出来
if command -v magick >/dev/null 2>&1; then
    magick assets/icon/logo.png -resize 512x512 "$STAGING/share/icons/hicolor/512x512/apps/$APP_ID.png"
else
    echo "   没有 ImageMagick，图标原样放（尺寸与目录名不符，不影响使用）"
    install -m 644 assets/icon/logo.png "$STAGING/share/icons/hicolor/512x512/apps/$APP_ID.png"
fi

if [ "$skip_data" -eq 0 ]; then
    if [ ! -f data/generated/dict.qj ]; then
        echo "没有 data/generated/dict.qj，先取产品数据（见 docs/notes/release.md），或者加 --skip-data" >&2
        exit 1
    fi
    echo "==> 摆随包数据"
    mkdir -p "$STAGING/share/qingjian/data/generated" "$STAGING/share/qingjian/assets"
    cp -r data/generated/. "$STAGING/share/qingjian/data/generated/"
    cp -r assets/emoji assets/levels "$STAGING/share/qingjian/assets/"
fi
if [ "$skip_model" -eq 0 ] && [ -f data/model/model.qjm ]; then
    echo "==> 摆本地整句模型"
    mkdir -p "$STAGING/share/qingjian/data/model"
    install -m 644 data/model/model.qjm "$STAGING/share/qingjian/data/model/"
fi

echo "==> flatpak-builder"
cp "$MANIFEST_SRC" "$OUT/$APP_ID.yml"
# --disable-rofiles-fuse：装成 flatpak 的 org.flatpak.Builder 在自己的沙箱里拿不到 /dev/fuse，
# rofiles-fuse 起不来（`fusermount: … can't send fuse fd`）。它只是构建期的写保护优化，关掉不影响产物。
builder --user --force-clean --disable-rofiles-fuse --repo="$REPO" "$OUT/build" "$OUT/$APP_ID.yml"

# 组件 XML：exec 必须是「怎么把引擎跑起来」的完整命令，在 Flatpak 里就是 flatpak run
echo "==> 生成给宿主机用的组件 XML"
target/release/qingjian-linux --ibus-xml "flatpak run --command=qingjian-linux $APP_ID" \
    > "$OUT/qingjian.xml"

if [ "$bundle" -eq 1 ]; then
    echo "==> 导出单文件"
    flatpak build-bundle "$REPO" "$OUT/qingjian.flatpak" "$APP_ID"
fi
if [ "$install" -eq 1 ]; then
    echo "==> 装到本用户"
    # 从本地 repo 装要先把它加成 remote：直接把路径递给 install 是不行的。
    # 本地构建没有签名，所以 --no-gpg-verify。
    flatpak remote-add --user --no-gpg-verify --if-not-exists qingjian-local "$REPO"
    flatpak install --user --reinstall --assumeyes --noninteractive qingjian-local "$APP_ID"
fi

echo
echo "==> 成品"
[ -f "$OUT/qingjian.flatpak" ] && ls -lh "$OUT/qingjian.flatpak" | awk '{print "   " $9 "  " $5}'
echo "   $OUT/repo（flatpak 仓库）"
echo "   $OUT/qingjian.xml（组件描述）"
cat <<HINT

装好之后还差两步，Flatpak 自己做不到：

  1) 把组件描述放进宿主机（ibus-daemon 只认这个目录，不看 Flatpak 导出的那份）：
       sudo install -Dm644 $OUT/qingjian.xml /usr/share/ibus/component/qingjian.xml
  2) 让 ibus 重扫并重启：
       ibus write-cache --system && ibus restart

**装成 --user 时留意**：flatpak run 是从 \$XDG_DATA_HOME/flatpak 找应用的，ibus-daemon 的环境里
那个变量得是正常值，否则它拉起引擎时报「app/... 未安装」，而且**只写在 daemon 自己的日志里**，
从输入法这边看只是「切不到引擎、超时」。桌面会话的 daemon 正常；环境被改过时装 --system 更稳。

然后在「设置 → 键盘 → 输入源」里加「青简」。设置界面：flatpak run $APP_ID
自检（沙箱里跑，看数据与总线找得到没有）：
       flatpak run --command=qingjian-linux $APP_ID --check
HINT
