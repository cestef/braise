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
		// First try to find the binary in the workspace
		const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
		if (workspaceFolder) {
			const workspaceLspPath = path.join(
				workspaceFolder.uri.fsPath,
				"target",
				"release",
				process.platform === "win32" ? "braise-lsp.exe" : "braise-lsp",
			);

			const debugLspPath = path.join(
				workspaceFolder.uri.fsPath,
				"target",
				"debug",
				process.platform === "win32" ? "braise-lsp.exe" : "braise-lsp",
			);

			// Check if release build exists
			if (this.fileExists(workspaceLspPath)) {
				return { command: workspaceLspPath };
			}

			// Check if debug build exists
			if (this.fileExists(debugLspPath)) {
				return { command: debugLspPath };
			}
		}

		// Try to find in PATH
		const binaryName =
			process.platform === "win32" ? "braise-lsp.exe" : "braise-lsp";
		return { command: binaryName };
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

			vscode.window.showInformationMessage(
				"Braise Language Server started successfully!",
			);
		} catch (error) {
			vscode.window.showErrorMessage(
				`Failed to start Braise Language Server: ${error}`,
			);

			// Show instructions for building the LSP server
			const buildAction = "Build LSP Server";
			const choice = await vscode.window.showWarningMessage(
				"Braise Language Server not found. Would you like to build it?",
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
		terminal.sendText("cargo build --release --bin braise-lsp");

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
