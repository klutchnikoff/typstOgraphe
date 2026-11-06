"""Native Git hooks: staged-file checks and isolated commit verification before push."""
import importlib.util
import os
import json
import shutil
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(*args, cwd=None, env=None, **options):
    program = shutil.which(args[0]) or args[0]
    return subprocess.run([program, *args[1:]], cwd=cwd or ROOT, env=env, check=True, **options)


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def staged():
    run('git', 'diff', '--cached', '--check')
    names = git('diff', '--cached', '--name-only', '--diff-filter=ACMR', '-z')
    for name in filter(None, names.decode().split('\0')):
        suffix = Path(name).suffix
        if suffix not in {'.rs', '.ts', '.js', '.mjs', '.css', '.json', '.py', '.sh'} and not name.startswith('.githooks/'):
            continue
        contents = git('show', ':' + name)
        if suffix == '.py':
            compile(contents, name, 'exec')
        elif suffix == '.sh' or name.startswith('.githooks/'):
            run('sh', '-n', input=contents)
        elif suffix == '.rs':
            run('rustfmt', '--edition', '2024', '--config', 'skip_children=true', '--check', input=contents)
        else:
            # Ask Prettier whether this path is ignored before reading the staged blob.
            info = run('npx', '--no-install', 'prettier', '--file-info', name,
                       stdout=subprocess.PIPE).stdout
            if not json.loads(info)['ignored']:
                run('npx', '--no-install', 'prettier', '--check', '--stdin-filepath', name, input=contents)


def tips(lines):
    seen = set()
    for line in lines:
        local_ref, oid, remote_ref, base = line.split()
        if set(oid) == {'0'}:  # branch/tag deletion
            continue
        commit = git('rev-parse', '--verify', oid + '^{commit}').decode().strip()
        if commit not in seen:
            seen.add(commit)
            yield commit, base


def push(lines):
    spec = importlib.util.spec_from_file_location('ci_plan', ROOT / '.github/scripts/ci_plan.py')
    planner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(planner)
    # Git exports repository-local variables to hooks. They must not redirect
    # commands executed inside the detached verification worktree.
    env = dict(os.environ)
    for name in git('rev-parse', '--local-env-vars').decode().splitlines():
        env.pop(name, None)
    env['CARGO_TARGET_DIR'] = str(Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve())
    env['TOG_SPIKE_BINARY'] = str(Path(env['CARGO_TARGET_DIR']) / 'recipes/debug' / ('typstographe.exe' if os.name == 'nt' else 'typstographe'))
    for commit, base in tips(lines):
        if set(base) == {'0'}:
            try:
                base = git('merge-base', commit, 'origin/main').decode().strip()
            except subprocess.CalledProcessError:
                base = ''
        paths = planner.changed_paths(base, head=commit, root=ROOT)
        if not planner.plan(paths)['code']:
            print('Documentation-only push: expensive local checks skipped.', flush=True)
            continue
        recipes = planner.browser_recipes(paths, ROOT) if paths is not None else None
        with tempfile.TemporaryDirectory(prefix='typstographe-pre-push-') as directory:
            snapshot = Path(directory) / 'source'
            run('git', 'worktree', 'add', '--detach', str(snapshot), commit)
            try:
                if not (snapshot / 'scripts/verify.py').exists():
                    raise RuntimeError('Commit the verification scripts before pushing this commit.')
                print(f'Checking immutable commit {commit[:12]} in {snapshot}', flush=True)
                run('npm', 'ci', '--prefer-offline', '--no-audit', cwd=snapshot, env=env)
                command = ['python3', 'scripts/verify.py', 'push']
                if recipes is not None:
                    command += ['--recipes', *recipes]
                run(*command, cwd=snapshot, env=env)
            finally:
                run('git', 'worktree', 'remove', '--force', str(snapshot))


if __name__ == '__main__':
    if sys.argv[1] == 'pre-commit':
        staged()
    elif sys.argv[1] == 'pre-push':
        push(sys.stdin)
    else:
        raise SystemExit('Expected pre-commit or pre-push')
