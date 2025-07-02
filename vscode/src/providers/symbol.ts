import * as vscode from "vscode";
import { BraiseParser } from "../parser";

export class BraiseSymbolProvider implements vscode.DocumentSymbolProvider {
	private parser = new BraiseParser();

	provideDocumentSymbols(
		document: vscode.TextDocument,
	): vscode.DocumentSymbol[] {
		const symbols: vscode.DocumentSymbol[] = [];
		const recipes = this.parser.parseRecipes(document.getText());

		for (const recipe of recipes) {
			const line = document.lineAt(recipe.lineNumber);
			const range = line.range;
			const selectionRange = new vscode.Range(
				recipe.lineNumber,
				line.text.indexOf(recipe.name),
				recipe.lineNumber,
				line.text.indexOf(recipe.name) + recipe.name.length,
			);

			const symbol = new vscode.DocumentSymbol(
				recipe.name,
				recipe.dependencies.length > 0
					? `→ [${recipe.dependencies.join(", ")}]`
					: "",
				vscode.SymbolKind.Function,
				range,
				selectionRange,
			);

			// Add parameter symbols as children
			for (const param of recipe.parameters) {
				const paramSymbol = new vscode.DocumentSymbol(
					param.name,
					param.type + (param.defaultValue ? ` = ${param.defaultValue}` : ""),
					vscode.SymbolKind.Variable,
					range,
					range,
				);
				symbol.children.push(paramSymbol);
			}

			symbols.push(symbol);
		}

		return symbols;
	}
}
