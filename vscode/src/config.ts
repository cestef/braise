import * as vscode from "vscode";

export class BraiseConfigManager {
	private config: vscode.WorkspaceConfiguration;

	constructor() {
		this.config = vscode.workspace.getConfiguration("braise");
	}

	refresh() {
		this.config = vscode.workspace.getConfiguration("braise");
	}

	isLSPEnabled(): boolean {
		return this.config.get("enableLSP", true);
	}
}
