import path = require("path");
import { workspace, ExtensionContext } from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  const command = context.asAbsolutePath(
		path.join('out', 'drupal_ls')
	);

  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  const serverOptions: ServerOptions = {
    command,
    args: [],
    transport: TransportKind.stdio,
  };

  // Options to control the language client
  const clientOptions: LanguageClientOptions = {
    // Register the server for plain text documents
    documentSelector: [{ scheme: "file", language: "php" }, { scheme: "file", language: "yaml" }],
    synchronize: {
      // Notify the server about file changes to '.php files contained in the workspace
      fileEvents: workspace.createFileSystemWatcher("**/.{php,yml}"),
    },
  };

  // Create the language client and start the client.
  client = new LanguageClient(
    "drupal_ls",
    "Drupal LS",
    serverOptions,
    clientOptions,
  );

  // Start the client. This will also launch the server
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
