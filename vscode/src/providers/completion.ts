import * as vscode from "vscode";

const PRIMITIVE_TYPES = ["bool", "int", "string"];
const TYPE_CONSTRUCTORS = [(t: string) => t, (t: string) => `[${t}]`];

export class BraiseCompletionProvider implements vscode.CompletionItemProvider {
	private keywords = [
		"recipe",
		"param",
		"if",
		"else",
		"run",
		"exit",
		"match",
		"for",
		"in",
		"async",
	];

	private types = TYPE_CONSTRUCTORS.flatMap((c) =>
		PRIMITIVE_TYPES.map((type) => c(type)),
	);

	provideCompletionItems(
		document: vscode.TextDocument,
		position: vscode.Position,
	): vscode.CompletionItem[] {
		const items: vscode.CompletionItem[] = [];
		const lineText = document.lineAt(position.line).text;
		const beforeCursor = lineText.substring(0, position.character);

		// Keyword completions
		if (this.shouldProvideKeywords(beforeCursor)) {
			items.push(
				...this.keywords.map((keyword) => {
					const item = new vscode.CompletionItem(
						keyword,
						vscode.CompletionItemKind.Keyword,
					);
					item.insertText = keyword;
					return item;
				}),
			);
		}

		// Type completions after ':'
		if (beforeCursor.includes("param") && beforeCursor.endsWith(": ")) {
			items.push(
				...this.types.map((type) => {
					const item = new vscode.CompletionItem(
						type,
						vscode.CompletionItemKind.TypeParameter,
					);
					item.insertText = type;
					return item;
				}),
			);
		}

		return items;
	}

	private shouldProvideKeywords(beforeCursor: string): boolean {
		return (
			beforeCursor.trim().length === 0 ||
			beforeCursor.endsWith(" ") ||
			beforeCursor.endsWith("\t") ||
			beforeCursor.endsWith("{") ||
			beforeCursor.endsWith("}")
		);
	}
}
