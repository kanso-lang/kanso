import * as vscode from 'vscode';
import * as fs from 'fs';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
  // Get server path from configuration or fallback to default
  const config = vscode.workspace.getConfiguration('kanso');
  const configuredPath = config.get<string>('server.path') || 'kanso-lsp';
  
  // Try to find the server binary in common locations
  const serverCommand = findKansoLspBinary(configuredPath, context);
  
  if (!serverCommand) {
    showSetupHelp(configuredPath);
    return;
  }

  console.log(`Using Kanso LSP server at: ${serverCommand}`);

  const serverOptions: ServerOptions = {
    command: serverCommand,
    args: [],
    transport: TransportKind.stdio
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'kanso' }],
    synchronize: {
      // Watch for changes to Kanso files and configuration
      fileEvents: [
        vscode.workspace.createFileSystemWatcher('**/*.ka'),
        vscode.workspace.createFileSystemWatcher('**/kanso.toml'),
        vscode.workspace.createFileSystemWatcher('**/Kanso.toml')
      ]
    },
    // Enable enhanced LSP features
    initializationOptions: {
      semanticHighlighting: config.get('semanticHighlighting.enabled', true),
      completion: config.get('completion.enabled', true),
      diagnostics: config.get('diagnostics.enabled', true)
    }
  };

  client = new LanguageClient(
    'kansoLanguageServer',
    'Kanso Language Server',
    serverOptions,
    clientOptions
  );

  // Register additional VS Code integrations
  context.subscriptions.push(
    // Restart command for development
    vscode.commands.registerCommand('kanso.restart', () => {
      if (client) {
        client.restart();
        vscode.window.showInformationMessage('Kanso Language Server restarted');
      }
    }),
    
    // Show server output for debugging
    vscode.commands.registerCommand('kanso.showOutput', () => {
      if (client) {
        client.outputChannel.show();
      }
    })
  );

  // Start the language server
  client.start().then(() => {
    vscode.window.showInformationMessage('Kanso Language Server started successfully');
  }, (error) => {
    vscode.window.showErrorMessage(`Failed to start Kanso Language Server: ${error}`);
  });
}

function findKansoLspBinary(configuredPath: string, context: vscode.ExtensionContext): string | null {
  // First priority: packaged binary with the extension
  const packagedBinary = vscode.Uri.joinPath(context.extensionUri, 'bin', 'kanso-lsp').fsPath;
  console.log(`Checking for packaged kanso-lsp at: ${packagedBinary}`);
  if (fs.existsSync(packagedBinary)) {
    console.log(`Found packaged kanso-lsp at: ${packagedBinary}`);
    return packagedBinary;
  }

  // If an absolute path is configured, check if it exists
  if (configuredPath.startsWith('/')) {
    if (fs.existsSync(configuredPath)) {
      return configuredPath;
    }
    console.log(`Configured absolute path does not exist: ${configuredPath}`);
    return null;
  }

  // Common locations to search for the binary
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  const workspaceRoot = workspaceFolder?.uri.fsPath;
  
  const searchPaths = [
    // Current workspace root + configured path
    workspaceRoot ? `${workspaceRoot}/${configuredPath}` : null,
    // Parent directory of workspace (for development)
    workspaceRoot ? `${workspaceRoot}/../${configuredPath}` : null,
    // Hardcoded development path
    '/Users/daniel/projects/kanso-lang/kanso-lsp',
    // Just the configured path (will use system PATH)
    configuredPath,
  ].filter(Boolean) as string[];

  // Check each path for existence
  for (const path of searchPaths) {
    console.log(`Checking for kanso-lsp at: ${path}`);
    if (fs.existsSync(path)) {
      console.log(`Found kanso-lsp at: ${path}`);
      return path;
    }
  }

  console.log(`kanso-lsp not found in any of these locations: ${searchPaths.join(', ')}`);
  return null;
}

function showSetupHelp(configuredPath: string) {
  const message = `Kanso Language Server (kanso-lsp) not found at "${configuredPath}".`;
  const options = ['Install Instructions', 'Configure Path', 'Download'];
  
  vscode.window.showErrorMessage(message, ...options).then(selection => {
    if (selection === 'Install Instructions') {
      vscode.window.showInformationMessage(
        'To install kanso-lsp:\n\n' +
        '1. Clone the Kanso repository\n' +
        '2. Run "make kanso-lsp" to build the binary\n' +
        '3. Either:\n' +
        '   - Add the binary to your PATH, or\n' +
        '   - Configure "kanso.server.path" in VS Code settings'
      );
    } else if (selection === 'Configure Path') {
      vscode.commands.executeCommand('workbench.action.openSettings', 'kanso.server.path');
    } else if (selection === 'Download') {
      vscode.env.openExternal(vscode.Uri.parse('https://github.com/kanso-lang/kanso'));
    }
  });
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}