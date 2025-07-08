import * as path from "node:path";
import * as vscode from "vscode";
import {
	LanguageClient,
	type LanguageClientOptions,
	type ServerOptions,
} from "vscode-languageclient/node";

export class BraiseLanguageClient {
	private client: LanguageClient;
	private context: vscode.ExtensionContext;

	constructor(context: vscode.ExtensionContext) {
		this.context = context;
		this.client = undefined as unknown as LanguageClient;
		this.setupClient();
	}

	private setupClient() {
		// Try to find the LSP server binary
		const serverOptions: ServerOptions = this.getServerOptions();

		const clientOptions: LanguageClientOptions = {
			documentSelector: [{ scheme: "file", language: "braise" }],
			synchronize: {
				fileEvents: vscode.workspace.createFileSystemWatcher("**/*.braise"),
			},
			outputChannelName: "Braise Language Server",
		};

		this.client = new LanguageClient(
			"braiseLanguageServer",
			"Braise Language Server",
			serverOptions,
			clientOptions,
		);
	}

	private getServerOptions(): ServerOptions {
		// First try to find the braise binary in the workspace
		const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
		if (workspaceFolder) {
			const workspaceBraisePath = path.join(
				workspaceFolder.uri.fsPath,
				"target",
				"release",
				process.platform === "win32" ? "braise.exe" : "braise",
			);

			const debugBraisePath = path.join(
				workspaceFolder.uri.fsPath,
				"target",
				"debug",
				process.platform === "win32" ? "braise.exe" : "braise",
			);

			// Check if release build exists
			if (this.fileExists(workspaceBraisePath)) {
				return { command: workspaceBraisePath, args: ["lsp"] };
			}

			// Check if debug build exists
			if (this.fileExists(debugBraisePath)) {
				return { command: debugBraisePath, args: ["lsp"] };
			}
		}

		// Try to find braise in PATH
		const binaryName = process.platform === "win32" ? "braise.exe" : "braise";
		return { command: binaryName, args: ["lsp"] };
	}

	private fileExists(filePath: string): boolean {
		try {
			const fs = require("node:fs");
			return fs.existsSync(filePath);
		} catch {
			return false;
		}
	}

	async start() {
		try {
			await this.client.start();

			this.context.subscriptions.push({
				dispose: () => this.stop(),
			});
		} catch (error) {
			vscode.window.showErrorMessage(
				`Failed to start Braise Language Server: ${error}`,
			);

			// Show instructions for building the LSP server
			const buildAction = "Build LSP Server";
			const choice = await vscode.window.showWarningMessage(
				"Braise CLI not found. Would you like to build it?",
				buildAction,
				"Cancel",
			);

			if (choice === buildAction) {
				this.buildLspServer();
			}
		}
	}

	private async buildLspServer() {
		const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
		if (!workspaceFolder) {
			vscode.window.showErrorMessage("No workspace folder found");
			return;
		}

		const terminal = vscode.window.createTerminal({
			name: "Build Braise LSP",
			cwd: workspaceFolder.uri.fsPath,
		});

		terminal.show();
		terminal.sendText("cargo build --release --bin braise");

		vscode.window.showInformationMessage(
			"Building Braise LSP Server... Check the terminal for progress.",
		);
	}

	stop(): Thenable<void> | undefined {
		if (!this.client) {
			return undefined;
		}
		return this.client.stop();
	}
}
