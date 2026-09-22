#!/usr/bin/env python3
# //! 安装文件清单与校验；不管理桌面服务、系统文件或用户数据。
# //! 装哪几支（fcitx5 插件 / IBus 前端 / systemd 单元）由 install.sh 决定，这里只按给的路径照做。
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import tarfile


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def parse_args():
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('action', choices=('install', 'uninstall'))
    parser.add_argument('prefix')
    parser.add_argument('--root', help='仓库根，install 时必给')
    parser.add_argument('--server', help='Server 可执行文件')
    parser.add_argument('--plugin', help='fcitx5 插件 qingjian.so；这一支不装就不给')
    parser.add_argument('--ibus', help='IBus 前端可执行文件；这一支不装就不给')
    parser.add_argument('--ibus-xml', help='IBus 组件 XML（install.sh 按安装路径生成好的）')
    parser.add_argument('--ibus-env', help='设 IBUS_COMPONENT_PATH 的 environment.d 片段（install.sh 生成好的）')
    parser.add_argument('--service', help='systemd 用户单元文件（同上，已填好路径）')
    parser.add_argument('--sample', action='store_true', help='只装样例词库，不校验产品数据')
    return parser.parse_args()


def product_data(root, resources, files):
    """产品数据按 tools/release/data.lock 校验后加入清单。"""
    generated = root / 'data/generated'
    dictionary = generated / 'dict.qj'
    archive = root / 'target/release-data/qingjian-data.tar.gz'
    lock = dict(line.strip().split(' = ', 1) for line in (root / 'tools/release/data.lock').read_text().splitlines() if ' = ' in line)
    if not dictionary.is_file() or not archive.is_file():
        raise SystemExit('缺少产品数据，请先运行 tools/release/data-fetch.sh，或用 --sample 体验样例词库')
    if digest(archive) != lock.get('qingjian-data.tar.gz'):
        raise SystemExit('产品数据包与 tools/release/data.lock 校验值不符')
    verified = {}
    with tarfile.open(archive) as bundle:
        for member in bundle.getmembers():
            if not member.isfile():
                continue
            source = (generated / member.name).resolve()
            if not source.is_relative_to(generated.resolve()):
                raise SystemExit('数据包路径越界')
            checksum = hashlib.file_digest(bundle.extractfile(member), 'sha256').hexdigest()
            if not source.is_file() or digest(source) != checksum:
                raise SystemExit(f'产品数据校验失败：{member.name}')
            verified[source] = checksum
    wanted = ('dict.qj', 'lm.qj', 'lm-unigram.tsv', 'lm-bigram.tsv', 'english.tsv')
    for source in generated.rglob('*'):
        # 点开头的是 macOS 打包混进来的 AppleDouble（`._dict.qj`），data-v1 发布包里有 19 个，不是产品数据
        if source.name.startswith('.'):
            continue
        if source.is_file() and (source.name in wanted or source.name.startswith('glossary-') or source.parent.name == 'dicts'):
            if source.resolve() not in verified:
                raise SystemExit(f'产品数据没有校验记录：{source}')
            files[resources / source.relative_to(root)] = source


def main():
    options = parse_args()
    prefix = Path(options.prefix)
    if not prefix.is_absolute() or prefix == Path('/'):
        raise SystemExit('需要非根绝对安装路径')
    prefix = prefix.resolve()
    manifest = prefix / 'share/qingjian/install-manifest.json'
    old = json.loads(manifest.read_text()) if manifest.is_file() else {}
    if options.action == 'uninstall':
        for filename, checksum in old.items():
            path = Path(filename)
            if path.is_file() and not path.is_symlink() and digest(path) == checksum:
                path.unlink()
            elif path.exists():
                print(f'保留已修改文件：{path}')
        manifest.unlink(missing_ok=True)
        return
    if not options.root or not options.server:
        raise SystemExit('install 要给 --root 与 --server')
    root = Path(options.root)
    data = Path(os.environ.get('XDG_DATA_HOME', str(Path.home() / '.local/share')))
    config = Path(os.environ.get('XDG_CONFIG_HOME', str(Path.home() / '.config')))
    for directory in (data, config):
        if not directory.is_absolute():
            raise SystemExit('XDG_DATA_HOME / XDG_CONFIG_HOME 必须是绝对路径')
    files = {prefix / 'share/licenses/qingjian/LICENSE': root / 'LICENSE',
             data / 'icons/hicolor/128x128/apps/qingjian.png': root / 'assets/icon/logo.png',
             prefix / 'bin/qingjian-linux-server': Path(options.server)}
    if options.plugin:
        files[prefix / 'lib/fcitx5/qingjian.so'] = Path(options.plugin)
        for kind in ('addon', 'inputmethod'):
            files[data / f'fcitx5/{kind}/qingjian.conf'] = root / f'apps/linux/fcitx5/data/{kind}/qingjian.conf'
    if options.ibus:
        files[prefix / 'bin/qingjian-linux-ibus'] = Path(options.ibus)
        # 组件 XML 里写死了可执行文件的绝对路径，所以由 install.sh 按 prefix 生成好再交过来
        files[data / 'ibus/component/qingjian.xml'] = Path(options.ibus_xml)
        # IBus 不读用户目录下的组件，只认 IBUS_COMPONENT_PATH；会话启动时由 systemd 从 environment.d 读进来
        files[config / 'environment.d/qingjian-ibus.conf'] = Path(options.ibus_env)
    if options.service:
        files[config / 'systemd/user/qingjian-server.service'] = Path(options.service)
    resources = prefix / 'share/qingjian/resources'
    for kind in ('sample', 'glossary', 'levels', 'emoji', 'symbol'):
        for source in (root / 'assets' / kind).rglob('*'):
            if source.is_file():
                files[resources / source.relative_to(root)] = source
    if not options.sample:
        product_data(root, resources, files)
    # 安装前先检查所有目标，避免覆盖其他来源的同名文件。
    for target in files:
        if target.is_symlink() or (target.exists() and (str(target) not in old or digest(target) != old[str(target)])):
            raise SystemExit(f'目标已存在且不属于本次安装：{target}')
    installed = {}
    for target, source in files.items():
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_name(target.name + '.qingjian-tmp')
        shutil.copy2(source, temporary)
        if target == data / 'fcitx5/addon/qingjian.conf':
            temporary.write_text(temporary.read_text().replace('Library=qingjian\n', f'Library={prefix}/lib/fcitx5/qingjian\n'))
        temporary.replace(target)
        installed[str(target)] = digest(target)
    for filename, checksum in old.items():
        path = Path(filename)
        if filename not in installed and path.is_file() and not path.is_symlink() and digest(path) == checksum:
            path.unlink()
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(json.dumps(installed, ensure_ascii=False, indent=2) + '\n')


if __name__ == '__main__':
    main()
