"""Shared, non-writing verification commands for CI and local hooks."""
import argparse
import os
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def commands(level, recipes=None):
    quick = [
        ['npm', 'run', 'check:format'],
        ['npx', '--no-install', 'prettier', '--config', 'app/.prettierrc.json', '--check', '.github/scripts/*.mjs'],
        ['cargo', 'fmt', '--all', '--check'],
        ['npx', '--no-install', 'tsc', '--noEmit', '-p', 'app/tsconfig.json'],
        ['python3', '-m', 'unittest', 'discover', '-s', 'scripts/tests'],
        ['python3', 'scripts/check_file_length.py'],
        ['node', '--test', '.github/scripts/release.test.mjs'],
    ]
    lint = [['python3', 'scripts/check_core_dependencies.py'], ['npm', 'run', 'build:web'],
            ['cargo', 'clippy', '--locked', '--workspace', '--all-targets',
             '--features', 'typstographe/recipe-driver', '--', '-D', 'warnings']]
    tests = [['npm', 'run', 'build:web'], ['npm', 'run', 'check:distribution'],
             ['npm', 'test'], ['cargo', 'test', '--locked', '--workspace',
                              '--features', 'typstographe/recipe-driver']]
    if level == 'quick':
        return quick
    if level == 'lint':
        return lint
    if level == 'test':
        return tests
    result = quick + lint + tests[1:] + [
        ['npm', 'run', 'build:recipe'],
        ['npm', 'run', 'test:contract', '--workspace', '@typstographe/app'],
        ['npx', '--no-install', '--prefix', 'app', 'playwright', 'install', 'chromium'],
    ]
    # Unknown change history runs all Chromium recipes instead of guessing.
    if recipes is None:
        result.append(['npm', 'run', 'test:browser', '--', '--project', 'chromium'])
    else:
        result.append(['npm', 'run', 'test:browser', '--', '--project', 'chromium', '--grep', '@smoke'])
        if recipes:
            result.append(['npm', 'run', 'test:browser', '--', '--project', 'chromium',
                           '--grep-invert', '@smoke', *recipes])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('level', choices=['quick', 'lint', 'test', 'push'])
    parser.add_argument('--recipes', nargs='*', default=None)
    parser.add_argument('--dry-run', action='store_true')
    args = parser.parse_args()
    env = dict(os.environ)
    env.setdefault('CARGO_BUILD_JOBS', '2')
    env.setdefault('TOG_BROWSER_PORT', '1437')
    for command in commands(args.level, args.recipes):
        print('+ ' + shlex.join(command), flush=True)
        if not args.dry_run:
            subprocess.run([shutil.which(command[0]) or command[0], *command[1:]], cwd=ROOT, env=env, check=True)


if __name__ == '__main__':
    main()
