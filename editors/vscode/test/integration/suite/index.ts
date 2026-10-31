import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import Mocha from 'mocha';

export function run(): Promise<void> {
  const mocha = new Mocha({ ui: 'bdd', color: true, timeout: 60_000 });
  for (const file of readdirSync(__dirname).filter((name) => name.endsWith('.test.js'))) {
    mocha.addFile(join(__dirname, file));
  }
  return new Promise((resolve, reject) => {
    mocha.run((failures) => (failures === 0 ? resolve() : reject(new Error(`${failures} tests failed`))));
  });
}
