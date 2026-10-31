// Which archangel binary the extension runs. Kept apart from the extension so the tests can run it
// without VS Code.
import { statSync } from 'node:fs';
import { delimiter, isAbsolute, join, resolve } from 'node:path';

/** Where the extension looks, in order. */
export interface Places {
  /** The `archangel.path` setting, empty when unset. */
  setting: string;
  /** The workspace folder, when one is open. */
  folder: string | undefined;
  /** The `PATH` environment variable. */
  pathVariable: string | undefined;
  platform: NodeJS.Platform;
}

export type Lookup =
  | { ok: true; path: string; from: string }
  | { ok: false; message: string };

const INSTALL =
  'Install it in this workspace with `npm install --save-dev @enhance-eng/archangel`, ' +
  'or set `archangel.path` to the binary.';

/**
 * The binary from the setting, else the workspace's `node_modules/.bin`, else `PATH`. A setting
 * that names no file is an error, never a reason to look elsewhere.
 */
export function findBinary(places: Places, isFile: (path: string) => boolean = fileExists): Lookup {
  const names = places.platform === 'win32' ? ['archangel.exe', 'archangel.cmd'] : ['archangel'];

  if (places.setting !== '') {
    if (!isAbsolute(places.setting) && places.folder === undefined) {
      return {
        ok: false,
        message: `The archangel.path setting, ${places.setting}, is relative, and no workspace folder is open.`,
      };
    }
    const path = places.folder === undefined ? places.setting : resolve(places.folder, places.setting);
    if (isFile(path)) return { ok: true, path, from: 'the archangel.path setting' };
    return { ok: false, message: `The archangel.path setting names ${path}, which is not a file.` };
  }

  const bin = places.folder === undefined ? undefined : join(places.folder, 'node_modules', '.bin');
  if (bin !== undefined) {
    const path = names.map((name) => join(bin, name)).find(isFile);
    if (path !== undefined) return { ok: true, path, from: "the workspace's node_modules/.bin" };
  }

  const dirs = (places.pathVariable ?? '').split(delimiter).filter((dir) => dir !== '');
  for (const dir of dirs) {
    const path = names.map((name) => join(dir, name)).find(isFile);
    if (path !== undefined) return { ok: true, path, from: 'PATH' };
  }

  const workspace =
    bin === undefined ? 'no workspace folder is open to look in node_modules/.bin' : `${bin} has no archangel`;
  return {
    ok: false,
    message:
      'Archangel could not find its binary. The archangel.path setting is empty, ' +
      `${workspace}, and no folder on PATH has it. ${INSTALL}`,
  };
}

function fileExists(path: string): boolean {
  return statSync(path, { throwIfNoEntry: false })?.isFile() ?? false;
}
