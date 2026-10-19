#!/usr/bin/env bash
# //! 用户目录安装默认面板链路；Server 由用户手动启动。
set -euo pipefail
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
install_prefix="$HOME/.local"
build_profile=release
cmake_type=Release
sample=false
fcitx4=false
while (($#)); do
  case "$1" in
    --prefix) install_prefix=${2:?--prefix 需要路径}; shift 2 ;;
    --debug) build_profile=debug; cmake_type=Debug; shift ;;
    --sample) sample=true; shift ;;
    --fcitx4) fcitx4=true; shift ;;
    --help) echo '用法：install.sh [--prefix 绝对用户目录] [--debug] [--sample] [--fcitx4]'; exit 0 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done
[[ "$install_prefix" = /* && "$install_prefix" != / ]] || { echo '需要非根绝对安装路径' >&2; exit 2; }
# 缺依赖先说清楚，别等编译到一半才报一长串错。
missing=()
for tool in cargo cmake c++ pkg-config python3; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if command -v pkg-config >/dev/null 2>&1; then
  modules=(openssl nlohmann_json)
  if $fcitx4; then modules+=(fcitx x11 xres); else modules+=(Fcitx5Core Fcitx5Utils Fcitx5Config); fi
  for module in "${modules[@]}"; do
    pkg-config --exists "$module" || missing+=("$module")
  done
fi
# 路径与 crates/bamboo-slip/src/fonts/linux.rs 的清单对应，改那边时同步这里。
# 没有系统 UI 字体设置程序起不来，算缺依赖；中文与 emoji 字体缺了照样能起，只提示。
any_file() {
  local path
  for path in "$@"; do [[ -f "$path" ]] && return 0; done
  return 1
}
any_file /usr/share/fonts/truetype/noto/NotoSans-Regular.ttf /usr/share/fonts/noto/NotoSans-Regular.ttf \
  '/usr/share/fonts/google-noto-vf/NotoSans[wght].ttf' /usr/share/fonts/google-noto/NotoSans-Regular.ttf \
  /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf /usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf \
  /usr/local/share/fonts/noto/NotoSans-Regular.ttf /usr/local/share/fonts/dejavu/DejaVuSans.ttf \
  || missing+=("系统 UI 字体（Noto Sans 或 DejaVu）")
any_file /usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc /usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc \
  /usr/share/fonts/google-noto-sans-cjk-vf-fonts/NotoSansCJK-VF.ttc /usr/share/fonts/google-noto-sans-cjk-fonts/NotoSansCJK-Regular.ttc \
  /usr/local/share/fonts/noto/NotoSansCJKsc-Regular.otf \
  || echo '提示：没找到中文字体（Noto Sans CJK），设置程序里的中文会画不出来' >&2
any_file /usr/share/fonts/truetype/noto/NotoColorEmoji.ttf /usr/share/fonts/noto/NotoColorEmoji.ttf \
  /usr/share/fonts/google-noto-color-emoji-fonts/NotoColorEmoji.ttf /usr/local/share/fonts/noto/NotoColorEmoji.ttf \
  || echo '提示：没找到 emoji 字体（Noto Color Emoji），表情会画不出来' >&2
if ((${#missing[@]})); then
  echo "缺少依赖：${missing[*]}" >&2
  if $fcitx4; then
    echo 'Debian / Ubuntu：sudo apt install cmake g++ pkg-config python3 libssl-dev fcitx-libs-dev libx11-dev libxres-dev nlohmann-json3-dev fonts-noto-core fonts-noto-cjk fonts-noto-color-emoji；Rust 见 https://rustup.rs' >&2
  else
    echo 'Debian / Ubuntu：sudo apt install cmake g++ pkg-config python3 libssl-dev libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev fonts-noto-core fonts-noto-cjk fonts-noto-color-emoji；Rust 见 https://rustup.rs' >&2
    echo 'FreeBSD：pkg install bash cmake pkgconf python3 fcitx5 nlohmann-json rustup-init（rustup-init 装 Rust）' >&2
  fi
  exit 1
fi
install_prefix=$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$install_prefix")
cargo_output=$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "${CARGO_TARGET_DIR:-$repo_root/target}")
cargo_args=(build --manifest-path "$repo_root/Cargo.toml" --target-dir "$cargo_output" -p qingjian-linux-server -p qingjian-settings --locked)
[[ "$build_profile" != release ]] || cargo_args+=(--release)
cargo "${cargo_args[@]}"
if $fcitx4; then
  flavor=fcitx4
  cmake_output="$repo_root/target/fcitx4-install-$build_profile"
  plugin=fcitx-qingjian.so
else
  flavor=fcitx5
  cmake_output="$repo_root/target/fcitx5-install-$build_profile"
  plugin=qingjian.so
fi
cmake_args=("-DCMAKE_BUILD_TYPE=$cmake_type")
if ! $fcitx4; then cmake_args+=(-DBUILD_TESTING=OFF); fi
cmake -S "$repo_root/apps/linux/$flavor" -B "$cmake_output" "${cmake_args[@]}"
cmake --build "$cmake_output" --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-2}"
python3 "$repo_root/apps/linux/scripts/files.py" install "$install_prefix" "$repo_root" "$cargo_output/$build_profile/qingjian-linux-server" "$cargo_output/$build_profile/qingjian-settings" "$cmake_output/$plugin" "$sample" "$flavor"
if $fcitx4; then
  printf '已安装。手动启动：%s/bin/qingjian-linux-server\n重启 Fcitx4，在配置工具添加“青简”。\n' "$install_prefix"
else
  printf '已安装。手动启动：%s/bin/qingjian-linux-server\n重启 Fcitx5，在配置工具取消“仅显示当前语言”后添加“青简”。\n' "$install_prefix"
fi
printf '设置：应用菜单里的“青简设置”，或 %s/bin/qingjian-settings\n' "$install_prefix"
