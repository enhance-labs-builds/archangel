import { strict as assert } from 'node:assert';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, dirname, join } from 'node:path';
import { findBinary, type Places } from '../../src/binary';

describe('findBinary', () => {
  let root: string;
  let folder: string;
  let pathDir: string;

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), 'archangel-vscode-'));
    folder = join(root, 'workspace');
    pathDir = join(root, 'path-bin');
    mkdirSync(folder);
    mkdirSync(pathDir);
  });

  afterEach(() => {
    rmSync(root, { recursive: true, force: true });
  });

  function binary(path: string): string {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, '');
    return path;
  }

  function places(overrides: Partial<Places> = {}): Places {
    return { setting: '', folder, pathVariable: pathDir, platform: 'darwin', ...overrides };
  }

  it('uses the setting before the workspace and PATH', () => {
    const set = binary(join(root, 'custom', 'archangel'));
    binary(join(folder, 'node_modules', '.bin', 'archangel'));
    binary(join(pathDir, 'archangel'));

    assert.deepEqual(findBinary(places({ setting: set })), {
      ok: true,
      path: set,
      from: 'the archangel.path setting',
    });
  });

  it('takes a relative setting from the workspace folder', () => {
    const set = binary(join(folder, 'tools', 'archangel'));

    assert.deepEqual(findBinary(places({ setting: 'tools/archangel' })), {
      ok: true,
      path: set,
      from: 'the archangel.path setting',
    });
  });

  it('refuses a setting that names no file, rather than looking elsewhere', () => {
    binary(join(folder, 'node_modules', '.bin', 'archangel'));
    binary(join(pathDir, 'archangel'));
    const missing = join(root, 'nowhere', 'archangel');

    assert.deepEqual(findBinary(places({ setting: missing })), {
      ok: false,
      message: `The archangel.path setting names ${missing}, which is not a file.`,
    });
  });

  it('refuses a setting that names a folder', () => {
    const dir = join(root, 'archangel');
    mkdirSync(dir);

    assert.equal(findBinary(places({ setting: dir })).ok, false);
  });

  it('refuses a relative setting when no workspace folder is open', () => {
    assert.deepEqual(findBinary(places({ setting: 'tools/archangel', folder: undefined })), {
      ok: false,
      message: 'The archangel.path setting, tools/archangel, is relative, and no workspace folder is open.',
    });
  });

  it("uses the workspace's node_modules/.bin before PATH", () => {
    const local = binary(join(folder, 'node_modules', '.bin', 'archangel'));
    binary(join(pathDir, 'archangel'));

    assert.deepEqual(findBinary(places()), {
      ok: true,
      path: local,
      from: "the workspace's node_modules/.bin",
    });
  });

  it('uses the first folder on PATH that has the binary', () => {
    const empty = join(root, 'empty');
    mkdirSync(empty);
    const second = join(root, 'second');
    binary(join(pathDir, 'archangel'));
    binary(join(second, 'archangel'));

    assert.deepEqual(findBinary(places({ pathVariable: [empty, '', pathDir, second].join(delimiter) })), {
      ok: true,
      path: join(pathDir, 'archangel'),
      from: 'PATH',
    });
  });

  it('looks for the .exe and the npm .cmd launcher on Windows', () => {
    const local = binary(join(folder, 'node_modules', '.bin', 'archangel.cmd'));
    binary(join(folder, 'node_modules', '.bin', 'archangel'));

    assert.deepEqual(findBinary(places({ platform: 'win32' })), {
      ok: true,
      path: local,
      from: "the workspace's node_modules/.bin",
    });
  });

  it('names the three places and how to install when nothing is found', () => {
    const bin = join(folder, 'node_modules', '.bin');

    assert.deepEqual(findBinary(places()), {
      ok: false,
      message:
        'Archangel could not find its binary. The archangel.path setting is empty, ' +
        `${bin} has no archangel, and no folder on PATH has it. ` +
        'Install it in this workspace with `npm install --save-dev @enhance-eng/archangel`, ' +
        'or set `archangel.path` to the binary.',
    });
  });

  it('says no workspace folder is open when nothing is found without one', () => {
    const lookup = findBinary(places({ folder: undefined, pathVariable: undefined }));

    assert.deepEqual(lookup, {
      ok: false,
      message:
        'Archangel could not find its binary. The archangel.path setting is empty, ' +
        'no workspace folder is open to look in node_modules/.bin, and no folder on PATH has it. ' +
        'Install it in this workspace with `npm install --save-dev @enhance-eng/archangel`, ' +
        'or set `archangel.path` to the binary.',
    });
  });
});
