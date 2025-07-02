import * as vscode from "vscode";
import type { BraiseConfigManager } from "./config";
import { BraiseParser } from "./parser";

export class BraiseCommandManager {
	private context: vscode.ExtensionContext;
	private config: BraiseConfigManager;
	private parser: BraiseParser;

	constructor(context: vscode.ExtensionContext, config: BraiseConfigManager) {
		this.context = context;
		this.config = config;
		this.parser = new BraiseParser();
	}

	registerCommands() {
		const commands = [
			this.registerRunRecipe(),
			this.registerNewRecipe(),
			this.registerRunRecipeWithParams(),
			this.registerValidateFile(),
			this.registerShowRecipeInfo(),
		];

		commands.forEach((command) => this.context.subscriptions.push(command));
	}

	private registerRunRecipe() {
		return vscode.commands.registerCommand("braise.runRecipe", async () => {
			const editor = vscode.window.activeTextEditor;
			if (!this.validateActiveEditor(editor)) return;

			const recipes = this.parser.parseRecipes(editor!.document.getText());
			if (recipes.length === 0) {
				vscode.window.showErrorMessage("No recipes found in this file");
				return;
			}

			const selectedRecipe = await vscode.window.showQuickPick(
				recipes.map((r) => ({
					label: r.name,
					description:
						r.dependencies.length > 0
							? `Depends on: ${r.dependencies.join(", ")}`
							: "",
					detail:
						r.parameters.length > 0
							? `Parameters: ${r.parameters.map((p) => p.name).join(", ")}`
							: "",
				})),
				{ placeHolder: "Select a recipe to run" },
			);

			if (selectedRecipe) {
				this.executeRecipe(editor!.document.uri.fsPath, selectedRecipe.label);
			}
		});
	}

	private registerNewRecipe() {
		return vscode.commands.registerCommand("braise.newRecipe", async () => {
			const editor = vscode.window.activeTextEditor;
			if (!this.validateActiveEditor(editor)) return;

			const recipeName = await vscode.window.showInputBox({
				prompt: "Enter recipe name",
				placeHolder: "my-recipe",
				validateInput: (value) => {
					if (!value || value.trim().length === 0) {
						return "Recipe name cannot be empty";
					}
					if (!/^[a-zA-Z][a-zA-Z0-9_-]*$/.test(value)) {
						return "Recipe name must start with a letter and contain only letters, numbers, hyphens, and underscores";
					}
					return null;
				},
			});

			if (recipeName) {
				const snippet = new vscode.SnippetString(
					`recipe "${recipeName}" {\n\t$0\n}\n`,
				);
				editor!.insertSnippet(snippet);
			}
		});
	}

	private registerRunRecipeWithParams() {
		return vscode.commands.registerCommand(
			"braise.runRecipeWithParams",
			async () => {
				const editor = vscode.window.activeTextEditor;
				if (!this.validateActiveEditor(editor)) return;

				const recipes = this.parser.parseRecipes(editor!.document.getText());
				if (recipes.length === 0) {
					vscode.window.showErrorMessage("No recipes found in this file");
					return;
				}

				const selectedRecipe = await vscode.window.showQuickPick(
					recipes.map((r) => ({
						label: r.name,
						description: `${r.parameters.length} parameter(s)`,
						recipe: r,
					})),
					{ placeHolder: "Select a recipe to run with parameters" },
				);

				if (selectedRecipe && selectedRecipe.recipe.parameters.length > 0) {
					const params = await this.collectParameters(
						selectedRecipe.recipe.parameters,
					);
					if (params) {
						this.executeRecipeWithParams(
							editor!.document.uri.fsPath,
							selectedRecipe.label,
							params,
						);
					}
				} else if (selectedRecipe) {
					this.executeRecipe(editor!.document.uri.fsPath, selectedRecipe.label);
				}
			},
		);
	}

	private registerValidateFile() {
		return vscode.commands.registerCommand("braise.validateFile", async () => {
			const editor = vscode.window.activeTextEditor;
			if (!this.validateActiveEditor(editor)) return;

			const validation = this.parser.validateFile(editor!.document.getText());
			if (validation.isValid) {
				vscode.window.showInformationMessage(
					`✓ Valid Braise file with ${validation.recipeCount} recipe(s)`,
				);
			} else {
				vscode.window.showErrorMessage(
					`✗ Invalid Braise file: ${validation.errors.join(", ")}`,
				);
			}
		});
	}

	private registerShowRecipeInfo() {
		return vscode.commands.registerCommand(
			"braise.showRecipeInfo",
			async () => {
				const editor = vscode.window.activeTextEditor;
				if (!this.validateActiveEditor(editor)) return;

				const recipes = this.parser.parseRecipes(editor!.document.getText());
				if (recipes.length === 0) {
					vscode.window.showErrorMessage("No recipes found in this file");
					return;
				}

				const selectedRecipe = await vscode.window.showQuickPick(
					recipes.map((r) => ({ label: r.name, recipe: r })),
					{ placeHolder: "Select a recipe to view info" },
				);

				if (selectedRecipe) {
					this.showRecipeInfoPanel(selectedRecipe.recipe);
				}
			},
		);
	}

	private validateActiveEditor(editor: vscode.TextEditor | undefined): boolean {
		if (!editor || editor.document.languageId !== "braise") {
			vscode.window.showErrorMessage("Please open a Braise file first");
			return false;
		}
		return true;
	}

	private async collectParameters(
		params: Array<{ name: string; type: string; defaultValue?: string }>,
	): Promise<Record<string, string> | null> {
		const collected: Record<string, string> = {};

		for (const param of params) {
			const value = await vscode.window.showInputBox({
				prompt: `Enter value for parameter "${param.name}" (${param.type})`,
				placeHolder: param.defaultValue || `Enter ${param.type} value`,
				value: param.defaultValue,
			});

			if (value === undefined) {
				return null; // User cancelled
			}

			collected[param.name] = value;
		}

		return collected;
	}

	private executeRecipe(filePath: string, recipeName: string) {
		const terminal = vscode.window.createTerminal(
			this.config.getDefaultTerminalName(),
		);
		const executable = this.config.getBraiseExecutablePath();
		terminal.sendText(`${executable} -f "${filePath}" "${recipeName}"`);
		terminal.show();
	}

	private executeRecipeWithParams(
		filePath: string,
		recipeName: string,
		params: Record<string, string>,
	) {
		const terminal = vscode.window.createTerminal(
			this.config.getDefaultTerminalName(),
		);
		const executable = this.config.getBraiseExecutablePath();
		const paramArgs = Object.entries(params)
			.map(([key, value]) => `--${key}="${value}"`)
			.join(" ");
		terminal.sendText(
			`${executable} -f "${filePath}" "${recipeName}" ${paramArgs}`,
		);
		terminal.show();
	}

	private showRecipeInfoPanel(recipe: any) {
		const panel = vscode.window.createWebviewPanel(
			"braiseRecipeInfo",
			`Recipe: ${recipe.name}`,
			vscode.ViewColumn.Beside,
			{},
		);

		panel.webview.html = this.getRecipeInfoHtml(recipe);
	}

	private getRecipeInfoHtml(recipe: any): string {
		return `
			<!DOCTYPE html>
			<html>
			<head>
				<style>
					body { font-family: var(--vscode-font-family); padding: 20px; }
					.param { margin: 10px 0; padding: 10px; background: var(--vscode-editor-background); border-radius: 4px; }
					.deps { color: var(--vscode-textLink-foreground); }
				</style>
			</head>
			<body>
				<h2>Recipe: ${recipe.name}</h2>
				${recipe.dependencies.length > 0 ? `<p class="deps">Dependencies: ${recipe.dependencies.join(", ")}</p>` : ""}
				<h3>Parameters:</h3>
				${recipe.parameters
					.map(
						(p: any) => `
					<div class="param">
						<strong>${p.name}</strong> (${p.type})
						${p.defaultValue ? `<br>Default: ${p.defaultValue}` : ""}
					</div>
				`,
					)
					.join("")}
			</body>
			</html>
		`;
	}
}
