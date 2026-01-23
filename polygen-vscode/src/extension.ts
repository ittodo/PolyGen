import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('polygen');
    const lspEnabled = config.get<boolean>('lsp.enabled', true);

    if (!lspEnabled) {
        console.log('PolyGen LSP is disabled');
        return;
    }

    const lspPath = config.get<string>('lsp.path', 'polygen-lsp');

    // Server options - run the LSP executable
    const serverOptions: ServerOptions = {
        run: {
            command: lspPath,
            transport: TransportKind.stdio,
        },
        debug: {
            command: lspPath,
            transport: TransportKind.stdio,
        },
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'poly' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.poly'),
        },
    };

    // Create and start the client
    client = new LanguageClient(
        'polygenLsp',
        'PolyGen Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client
    client.start().then(() => {
        console.log('PolyGen LSP client started');
    }).catch((error) => {
        console.error('Failed to start PolyGen LSP client:', error);
        vscode.window.showWarningMessage(
            `PolyGen LSP failed to start. Make sure 'polygen-lsp' is installed and in PATH. Error: ${error.message}`
        );
    });

    context.subscriptions.push({
        dispose: () => {
            if (client) {
                client.stop();
            }
        },
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
