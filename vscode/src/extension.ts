import * as vscode from "vscode";
import { BraiseLanguageClient } from "./client";
import { BraiseCommandManager } from "./commands";
import { BraiseConfigManager } from "./config";
import { BraiseProviderManager } from "./providers/index";

let languageClient: BraiseLanguageClient;
let commandManager: BraiseCommandManager;
let providerManager: BraiseProviderManager;
let configManager: BraiseConfigManager;

export function activate(context: vscode.ExtensionContext) {
	console.log("Braise language extension is now active!");

	configManager = new BraiseConfigManager();
	commandManager = new BraiseCommandManager(context, configManager);
	providerManager = new BraiseProviderManager(context);

	if (configManager.isLSPEnabled()) {
		languageClient = new BraiseLanguageClient(context);
		languageClient.start();
	}

	commandManager.registerCommands();
	providerManager.registerProviders();

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
