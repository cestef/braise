import * as path from "node:path";
import * as vscode from "vscode";
import {
	LanguageClient,
	type LanguageClientOptions,
	type ServerOptions,
	TransportKind,
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
		const serverModule = this.context.asAbsolutePath(
			path.join("server", "out", "server.js"),
		);

		const debugOptions = { execArgv: ["--nolazy", "--inspect=6009"] };
		const serverOptions: ServerOptions = {
			run: { module: serverModule, transport: TransportKind.ipc },
			debug: {
				module: serverModule,
				transport: TransportKind.ipc,
				options: debugOptions,
			},
		};

		const clientOptions: LanguageClientOptions = {
			documentSelector: [{ scheme: "file", language: "braise" }],
			synchronize: {
				fileEvents: vscode.workspace.createFileSystemWatcher("**/*.braise"),
			},
		};

		this.client = new LanguageClient(
			"braiseLanguageServer",
			"Braise Language Server",
			serverOptions,
			clientOptions,
		);
	}

	start() {
		this.client.start();

		this.context.subscriptions.push({
			dispose: () => this.stop(),
		});
	}

	stop(): Thenable<void> | undefined {
		if (!this.client) {
			return undefined;
		}
		return this.client.stop();
	}
}
