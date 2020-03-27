# run this script under the folder which contains a `Cargo.toml` file
# $ python3 format-toml.py
#
# or specify the toml file path in the command line
# $ python3 format-toml.py frame/staking/Cargo.toml
#
# don't worry about the origin toml file will be backed up to `Cargo.toml.bak`

import re
from os import getcwd
from shutil import copyfile
from sys import argv


class Dep:
    re_simple_dep = re.compile(r'^(.+?) *?= *?(".+?")$')
    re_alias = re.compile(r'^(.+?) *?=')
    re_package = re.compile(r'package *?= *?(".+?")')
    re_version = re.compile(r'version *?= *?(".+?")')
    re_optional = re.compile(r'optional *?= *?true')
    re_default_features = re.compile(r'default-features *?= *?false')
    re_features = re.compile(r'features *?= *?\[(.+?)\]')
    re_path = re.compile(r'path *?= *?(".+?")')
    re_git = re.compile(r'git *?= *?(".+?")')
    re_branch = re.compile(r'branch *?= *?(".+?")')
    re_tag = re.compile(r'tag *?= *?(".+?")')

    def __init__(self, alias: str) -> object:
        self.alias = alias
        self.package = None
        self.version = None
        self.optional = None
        self.default_features = None
        self.features = None
        self.path = None
        self.git = None
        self.branch = None
        self.tag = None

    @classmethod
    def from_str(cls, s: str) -> object:
        simple_dep = cls.re_simple_dep.match(s)
        if simple_dep:
            dep = Dep(simple_dep[1])
            dep.version = simple_dep[2]
            return dep

        dep = Dep(cls.re_alias.search(s)[1])

        package = cls.re_package.search(s)
        if package:
            dep.package = package[1]

        version = cls.re_version.search(s)
        if version:
            dep.version = version[1]

        optional = cls.re_optional.search(s)
        if optional:
            dep.optional = 'true'

        default_features = cls.re_default_features.search(s)
        if default_features:
            dep.default_features = 'false'

        features = cls.re_features.search(s)
        if features:
            dep.features = ''.join([
                '[',
                ', '.join([feature.strip() for feature in features[1].split(',')]),
                ']'
            ])

        path = cls.re_path.search(s)
        if path:
            dep.path = path[1]

        git = cls.re_git.search(s)
        if git:
            dep.git = git[1]

        branch = cls.re_branch.search(s)
        if branch:
            dep.branch = branch[1]

        tag = cls.re_tag.search(s)
        if tag:
            dep.tag = tag[1]

        return dep


def sort_deps(deps: [Dep]) -> ([Dep], [Dep], [Dep], [Dep]):
    crates = []
    darwinia = []
    github = []
    substrate = []
    for dep in sorted(deps, key=lambda dep: dep.alias):
        if dep.path:
            darwinia.append(dep)
        elif dep.git:
            if 'substrate.git' in dep.git:
                substrate.append(dep)
            else:
                github.append(dep)
        else:
            crates.append(dep)
    return crates, darwinia, github, substrate


def format_deps(deps: [Dep]) -> str:
    return '\n'.join(map(
        lambda dep: ''.join([
            dep.alias,
            ' = { ',
            ', '.join(map(
                lambda t: ''.join([t[0], ' = ', ''.join(t[1])]),
                filter(
                    lambda t: t[1], zip(
                        [
                            'package', 'version', 'optional', 'default-features',
                            'features', 'path', 'git', 'branch', 'tag',
                        ],
                        [
                            dep.package, dep.version, dep.optional, dep.default_features,
                            dep.features, dep.path, dep.git, dep.branch, dep.tag,
                        ],
                    ),
                ),
            )),
            ' }',
        ]),
        deps,
    ))


def write_deps(f: object, *deps: ([Dep], [Dep], [Dep], [Dep])) -> str:
    crates, darwinia, github, substrate = deps
    for deps, source in zip(
        [crates, darwinia, github, substrate],
        ['# crates', '# darwinia', '# github', '# substrate']
    ):
        if deps:
            c = source
            f.write('\n')
            f.write(c)
            f.write('\n')
            print(c)

            c = format_deps(deps)
            f.write(c)
            print(c)


toml_path = ''.join([getcwd(), '/Cargo.toml'])
if len(argv) > 1:
    toml_path = argv[1]

pkg = []
under_pkg = False

deps = []
under_deps = False

dev_deps = []
under_dev_deps = False

build_deps = []
under_build_deps = False

feats = {}
curr_feat = None
under_feats = False
is_std_feat = False
is_multi_line_feat = False
is_multi_line_feat_start = False

bench = []
under_bench = False

with open(toml_path, 'r') as file:
    for line in file:
        line = line.strip()

        if line == '' or line.startswith('#'):
            continue

        elif line == '[package]':
            under_pkg = True
            under_deps = False
            under_dev_deps = False
            under_build_deps = False
            under_feats = False
            under_bench = False
            continue

        elif line == '[dependencies]':
            under_pkg = False
            under_deps = True
            under_dev_deps = False
            under_build_deps = False
            under_feats = False
            under_bench = False
            continue

        elif line == '[dev-dependencies]':
            under_pkg = False
            under_deps = False
            under_dev_deps = True
            under_build_deps = False
            under_feats = False
            under_bench = False
            continue

        elif line == '[build-dependencies]':
            under_pkg = False
            under_deps = False
            under_dev_deps = False
            under_build_deps = True
            under_feats = False
            under_bench = False
            continue

        elif line == '[features]':
            under_pkg = False
            under_deps = False
            under_dev_deps = False
            under_build_deps = False
            under_feats = True
            under_bench = False
            continue

        elif line == '[[bench]]':
            under_pkg = False
            under_deps = False
            under_dev_deps = False
            under_build_deps = False
            under_feats = False
            under_bench = True

        elif under_pkg:
            pkg.append(line)

        elif under_deps:
            dep = Dep.from_str(line)
            deps.append(dep)

        elif under_dev_deps:
            dep = Dep.from_str(line)
            dev_deps.append(dep)

        elif under_build_deps:
            dep = Dep.from_str(line)
            build_deps.append(dep)

        elif under_feats:
            if not is_multi_line_feat:
                single_line = re.compile(r'(.+?) *?= *?\[(.*?)\]')
                curr_feat = single_line.match(line)
                if curr_feat:
                    is_multi_line_feat = False
                    is_multi_line_feat_start = True
                    feats[curr_feat[1]] = [
                        sub_feat.strip()
                        for sub_feat in curr_feat[2].split(',')
                    ]
                    continue
                else:
                    is_multi_line_feat = True

            if is_multi_line_feat_start:
                is_multi_line_feat_start = False
                curr_feat = re.match(r'(.+?) *?= *?\[', line)[1]
                if curr_feat == 'std':
                    is_std_feat = True
                else:
                    feats[curr_feat] = []
            elif line == ']':
                is_std_feat = False
                is_multi_line_feat = False
                is_multi_line_feat_start = True
            else:
                if not is_std_feat:
                    feats[curr_feat].append(re.match('(".+?")', line)[1])

        elif under_bench:
            bench.append(line)

copyfile(toml_path, ''.join([toml_path, '.bak']))

with open(toml_path, 'w+') as f:
    c = '[package]'
    f.write(c)
    f.write('\n')
    print(c)

    c = '\n'.join(pkg)
    f.write(c)
    print(c)

    if deps:
        f.write('\n\n')
        print()

        c = '[dependencies]'
        f.write(c)
        print(c)

        crates, darwinia, github, substrate = sort_deps(deps)
        write_deps(f, crates, darwinia, github, substrate)

    if dev_deps:
        f.write('\n\n')
        print()

        c = '[dev-dependencies]'
        f.write(c)
        print(c)

        write_deps(f, *sort_deps(dev_deps))

    if build_deps:
        f.write('\n\n')
        print()

        c = '[build-dependencies]'
        f.write(c)
        print(c)

        write_deps(f, *sort_deps(build_deps))

    if feats:
        f.write('\n\n')
        print()

        c = '[features]'
        f.write(c)
        f.write('\n')
        print(c)

        sorted_feats = []
        for (k, v) in feats.items():
            v.sort()
            sorted_feats.append(''.join([k, ' = [', ', '.join(v), ']']))
        sorted_feats.append(''.join([
            'std = [\n',
            ',\n'.join(map(
                lambda t: '\n'.join([
                    ''.join(['\t', t[1]]),
                    ',\n'.join(map(
                        lambda dep: ''.join(['\t"', dep.alias, '/std' if dep.default_features else '', '"']),
                        filter(lambda dep: dep.default_features or dep.optional, t[0]),
                    )),
                ]),
                filter(
                    lambda t: any(dep.default_features or dep.optinal for dep in t[0]),
                    zip(
                        [crates, darwinia, github, substrate],
                        ['# crates', '# darwinia', '# github', '# substrate'],
                    ),
                ),
            )),
            ',\n]',
        ]))
        sorted_feats.sort()
        c = '\n'.join(sorted_feats)
        f.write(c)
        print(c)

    if bench:
        f.write('\n\n')
        print()

        c = '[[bench]]'
        f.write(c)
        f.write('\n')
        print(c)

        c = '\n'.join(bench)
        f.write(c)
        print(c)

    f.write('\n')
