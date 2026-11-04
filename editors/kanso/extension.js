// The language client: starts `kanso lsp` for .kso files. It needs
// vscode-languageclient (`npm install` in this directory, once); without it
// the grammar still works and the server simply is not started.
const vscode = require('vscode');

let client = null;

function activate(context) {
  let lc;
  try {
    lc = require('vscode-languageclient/node');
  } catch (missing) {
    vscode.window.showInformationMessage(
      'kanso: run `npm install` in editors/kanso to start the language server');
    return;
  }
  const command = vscode.workspace.getConfiguration('kanso').get('server.path') || 'kanso';
  client = new lc.LanguageClient(
    'kanso',
    'kanso',
    { command, args: ['lsp'] },
    { documentSelector: [{ scheme: 'file', language: 'kanso' }] },
  );
  context.subscriptions.push({ dispose: () => client && client.stop() });
  client.start();
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
