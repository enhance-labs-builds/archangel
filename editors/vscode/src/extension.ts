import * as vscode from 'vscode';
import { LanguageClient, type LanguageClientOptions, type ServerOptions } from 'vscode-languageclient/node';
import { findBinary } from './binary';

let output: vscode.LogOutputChannel;
let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  output = vscode.window.createOutputChannel('Archangel', { log: true });
  context.subscriptions.push(
    output,
    vscode.commands.registerCommand('archangel.restart', async () => {
      await stop();
      await start();
    }),
  );
  await start();
}

export async function deactivate(): Promise<void> {
  await stop();
}

async function start(): Promise<void> {
  const folder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const lookup = findBinary({
    setting: vscode.workspace.getConfiguration('archangel').get<string>('path', ''),
    folder,
    pathVariable: process.env['PATH'],
    platform: process.platform,
  });
  if (!lookup.ok) {
    output.error(lookup.message);
    void vscode.window.showErrorMessage(lookup.message);
    return;
  }
  output.info(`Starting ${lookup.path} lsp, found in ${lookup.from}.`);

  // Windows runs a .cmd launcher, as npm installs into node_modules/.bin, only through a shell,
  // which splits an unquoted path at its spaces.
  const shell = lookup.path.endsWith('.cmd');
  const server: ServerOptions = {
    command: shell ? `"${lookup.path}"` : lookup.path,
    args: ['lsp'],
    options: { shell, ...(folder === undefined ? {} : { cwd: folder }) },
  };
  const options: LanguageClientOptions = {
    documentSelector: ['typescript', 'typescriptreact', 'javascript', 'javascriptreact'].map(
      (language) => ({ scheme: 'file', language }),
    ),
    outputChannel: output,
  };
  client = new LanguageClient('archangel', 'Archangel', server, options);
  await client.start();
}

async function stop(): Promise<void> {
  try {
    await client?.stop();
  } catch (error) {
    // The server can exit before the client has sent its last message, and a restart must still
    // start the new one.
    output.error(`Stopping the server failed: ${String(error)}`);
  }
  client = undefined;
}
