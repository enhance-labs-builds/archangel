import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';

function archangelDiagnostics(uri: vscode.Uri): vscode.Diagnostic[] {
  return vscode.languages.getDiagnostics(uri).filter((d) => d.source === 'archangel');
}

/** Resolves once the file's archangel diagnostics pass `done`, checking now and after each change. */
function diagnosticsWhen(uri: vscode.Uri, done: (d: vscode.Diagnostic[]) => boolean): Promise<vscode.Diagnostic[]> {
  return new Promise((resolve) => {
    const check = (): void => {
      const diagnostics = archangelDiagnostics(uri);
      if (!done(diagnostics)) return;
      subscription.dispose();
      resolve(diagnostics);
    };
    const subscription = vscode.languages.onDidChangeDiagnostics(check);
    check();
  });
}

describe('the archangel extension', () => {
  const folder = vscode.workspace.workspaceFolders?.[0]?.uri;
  assert.ok(folder, 'VS Code opened no workspace folder');
  const violating = vscode.Uri.joinPath(folder, 'src', 'shared', 'name.ts');

  it('publishes the layers diagnostic for a file that imports an earlier layer', async () => {
    await vscode.window.showTextDocument(violating);

    const [diagnostic, ...rest] = await diagnosticsWhen(violating, (d) => d.length > 0);

    assert.deepEqual(rest, []);
    assert.ok(diagnostic);
    assert.equal(diagnostic.code, 'layers');
    assert.equal(diagnostic.severity, vscode.DiagnosticSeverity.Error);
    assert.deepEqual(
      [diagnostic.range.start.line, diagnostic.range.start.character, diagnostic.range.end.line, diagnostic.range.end.character],
      [0, 21, 0, 34],
    );
    assert.match(diagnostic.message, /"shared" may not import "app"/);
  });

  it('restarts the server with the current archangel.path on Archangel: Restart Server', async () => {
    const settings = vscode.workspace.getConfiguration('archangel');
    const binary = settings.get<string>('path');
    assert.ok(binary);
    await diagnosticsWhen(violating, (d) => d.length > 0);

    await settings.update('path', `${binary}-missing`, vscode.ConfigurationTarget.Global);
    await vscode.commands.executeCommand('archangel.restart');
    assert.deepEqual(archangelDiagnostics(violating), []);

    await settings.update('path', binary, vscode.ConfigurationTarget.Global);
    await vscode.commands.executeCommand('archangel.restart');
    const diagnostics = await diagnosticsWhen(violating, (d) => d.length > 0);
    assert.deepEqual(diagnostics.map((d) => d.code), ['layers']);
  });
});
