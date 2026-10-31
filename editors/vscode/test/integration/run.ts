// Starts VS Code on the fixture workspace with archangel.path set to the debug binary, and runs the
// suite inside it.
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { runTests } from '@vscode/test-electron';

const extension = resolve(__dirname, '../../..');
const binary = process.env['ARCHANGEL_BINARY'] ?? resolve(extension, '../../target/debug/archangel');
// The oldest VS Code that engines.vscode accepts, and the one people run today.
const versions = ['1.91.0', 'stable'];

async function main(): Promise<void> {
  if (!existsSync(binary)) {
    throw new Error(`No archangel binary at ${binary}. Build it with \`cargo build -p archangel\`, or set ARCHANGEL_BINARY.`);
  }
  for (const version of versions) {
    const userData = mkdtempSync(join(tmpdir(), 'archangel-vscode-user-'));
    try {
      // A copy, so that a cargo build during the run cannot swap the binary under the server.
      const copy = join(userData, 'archangel');
      copyFileSync(binary, copy);
      mkdirSync(join(userData, 'User'));
      writeFileSync(join(userData, 'User', 'settings.json'), JSON.stringify({ 'archangel.path': copy }));
      await runTests({
        version,
        extensionDevelopmentPath: extension,
        extensionTestsPath: resolve(__dirname, 'suite'),
        launchArgs: [
          join(extension, 'test', 'fixture'),
          '--user-data-dir',
          userData,
          '--disable-extensions',
          '--disable-workspace-trust',
          '--skip-welcome',
        ],
      });
    } finally {
      rmSync(userData, { recursive: true, force: true });
    }
  }
}

main().catch((error: unknown) => {
  console.error('The integration tests failed:', error);
  process.exit(1);
});
