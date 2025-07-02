import * as path from "node:path";
import * as vscode from "vscode";
import {
	LanguageClient,
	type LanguageClientOptions,
	type ServerOptions,
	TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
	console.log("Braise language extension is now active!");

	registerCommands(context);

	const config = vscode.workspace.getConfiguration("braise");
	if (config.get("enableLSP", true)) {
		startLanguageServer(context);
	}

	registerProviders(context);
}

function registerCommands(context: vscode.ExtensionContext) {
	const runRecipeCommand = vscode.commands.registerCommand(
		"braise.runRecipe",
		async () => {
			const editor = vscode.window.activeTextEditor;
			if (!editor || editor.document.languageId !== "braise") {
				vscode.window.showErrorMessage("Please open a Braise file first");
				return;
			}

			const text = editor.document.getText();
			const recipeMatches = text.matchAll(/recipe\s+"([^"]+)"/g);
			const recipes = Array.from(recipeMatches, (m) => m[1]);

			if (recipes.length === 0) {
				vscode.window.showErrorMessage("No recipes found in this file");
				return;
			}

			const selectedRecipe = await vscode.window.showQuickPick(recipes, {
				placeHolder: "Select a recipe to run",
			});

			if (selectedRecipe) {
				const terminal = vscode.window.createTerminal("Braise");
				const filePath = editor.document.uri.fsPath;
				terminal.sendText(`braise -f ${filePath} ${selectedRecipe}`);
				terminal.show();
			}
		},
	);

	context.subscriptions.push(runRecipeCommand);

	const newRecipeCommand = vscode.commands.registerCommand(
		"braise.newRecipe",
		async () => {
			const editor = vscode.window.activeTextEditor;
			if (!editor || editor.document.languageId !== "braise") {
				vscode.window.showErrorMessage("Please open a Braise file first");
				return;
			}

			const recipeName = await vscode.window.showInputBox({
				prompt: "Enter recipe name",
				placeHolder: "my-recipe",
			});

			if (recipeName) {
				const snippet = new vscode.SnippetString(
					`recipe "${recipeName}" {\n\t$0\n}\n`,
				);
				editor.insertSnippet(snippet);
			}
		},
	);

	context.subscriptions.push(newRecipeCommand);
}

function startLanguageServer(context: vscode.ExtensionContext) {
	const serverModule = context.asAbsolutePath(
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

	client = new LanguageClient(
		"braiseLanguageServer",
		"Braise Language Server",
		serverOptions,
		clientOptions,
	);

	client.start();

	context.subscriptions.push({
		dispose: () => {
			if (client) {
				client.stop();
			}
		},
	});
}

function registerProviders(context: vscode.ExtensionContext) {
	const recipeCompletionProvider =
		vscode.languages.registerCompletionItemProvider("braise", {
			provideCompletionItems(document: vscode.TextDocument) {
				const text = document.getText();
				const recipeMatches = text.matchAll(/recipe\s+"([^"]+)"/g);
				const recipes = Array.from(recipeMatches, (m) => m[1]);

				return recipes.map((recipe) => {
					const item = new vscode.CompletionItem(
						recipe,
						vscode.CompletionItemKind.Function,
					);
					item.detail = "Recipe";
					item.documentation = `Recipe: ${recipe}`;
					return item;
				});
			},
		});

	context.subscriptions.push(recipeCompletionProvider);

	const symbolProvider = vscode.languages.registerDocumentSymbolProvider(
		"braise",
		{
			provideDocumentSymbols(
				document: vscode.TextDocument,
			): vscode.DocumentSymbol[] {
				const symbols: vscode.DocumentSymbol[] = [];
				const text = document.getText();
				const lines = text.split("\n");

				for (let i = 0; i < lines.length; i++) {
					const line = lines[i];
					const recipeMatch = line.match(/recipe\s+"([^"]+)"/);

					if (recipeMatch) {
						const recipeName = recipeMatch[1];
						const range = new vscode.Range(i, 0, i, line.length);
						const selectionRange = new vscode.Range(
							i,
							recipeMatch.index || 0,
							i,
							(recipeMatch.index || 0) + recipeMatch[0].length,
						);

						const symbol = new vscode.DocumentSymbol(
							recipeName,
							"Recipe",
							vscode.SymbolKind.Function,
							range,
							selectionRange,
						);

						symbols.push(symbol);
					}
				}

				return symbols;
			},
		},
	);

	context.subscriptions.push(symbolProvider);

	const foldingProvider = vscode.languages.registerFoldingRangeProvider(
		"braise",
		{
			provideFoldingRanges(
				document: vscode.TextDocument,
			): vscode.FoldingRange[] {
				const foldingRanges: vscode.FoldingRange[] = [];
				const text = document.getText();
				const lines = text.split("\n");

				const stack: number[] = [];

				for (let i = 0; i < lines.length; i++) {
					const line = lines[i].trim();

					if (line.includes("{")) {
						stack.push(i);
					}

					if (line.includes("}") && stack.length > 0) {
						const start = stack.pop();
						if (start !== undefined) {
							foldingRanges.push(new vscode.FoldingRange(start, i));
						}
					}
				}

				return foldingRanges;
			},
		},
	);

	context.subscriptions.push(foldingProvider);
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}
