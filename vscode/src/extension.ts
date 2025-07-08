import * as vscode from "vscode";
import { BraiseLanguageClient } from "./client";
import { BraiseConfigManager } from "./config";

let languageClient: BraiseLanguageClient;
let configManager: BraiseConfigManager;

export function activate(context: vscode.ExtensionContext) {
	console.log("Braise language extension is now active!");

	configManager = new BraiseConfigManager();

	if (configManager.isLSPEnabled()) {
		languageClient = new BraiseLanguageClient(context);
		languageClient.start();
	}

	context.subscriptions.push(
		vscode.commands.registerCommand("braise.restartLSP", async () => {
			if (languageClient) {
				await languageClient.stop();
				vscode.window.showInformationMessage(
					"Restarting Braise Language Server...",
				);
				await languageClient.start();
			} else {
				vscode.window.showInformationMessage(
					"Language Server is not running. Starting...",
				);
				languageClient = new BraiseLanguageClient(context);
				await languageClient.start();
			}
		}),
	);

	context.subscriptions.push(
		vscode.workspace.onDidChangeConfiguration((e) => {
			if (e.affectsConfiguration("braise")) {
				handleConfigurationChange(context);
			}
		}),
	);
}

function handleConfigurationChange(ctx: vscode.ExtensionContext) {
	configManager.refresh();

	const lspEnabled = configManager.isLSPEnabled();
	if (lspEnabled && !languageClient) {
		languageClient = new BraiseLanguageClient(ctx);
		languageClient.start();
	} else if (!lspEnabled && languageClient) {
		languageClient.stop();
		languageClient = undefined as unknown as BraiseLanguageClient;
	}
}

export function deactivate(): Thenable<void> | undefined {
	return languageClient?.stop();
}
