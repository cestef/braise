import * as vscode from "vscode";
import { BraiseCompletionProvider } from "./completion";
import { BraiseFoldingProvider } from "./folding";
import { BraiseSymbolProvider } from "./symbol";

export class BraiseProviderManager {
	private context: vscode.ExtensionContext;

	constructor(context: vscode.ExtensionContext) {
		this.context = context;
	}

	registerProviders() {
		const providers = [
			vscode.languages.registerDocumentSymbolProvider(
				"braise",
				new BraiseSymbolProvider(),
			),
			vscode.languages.registerFoldingRangeProvider(
				"braise",
				new BraiseFoldingProvider(),
			),
			vscode.languages.registerCompletionItemProvider(
				"braise",
				new BraiseCompletionProvider(),
				".",
				"$",
				"{",
			),
		];

		providers.forEach((provider) => this.context.subscriptions.push(provider));
	}
}
