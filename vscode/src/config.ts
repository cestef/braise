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

	getBraiseExecutablePath(): string {
		return this.config.get("executablePath", "braise");
	}

	getDefaultTerminalName(): string {
		return this.config.get("terminalName", "Braise");
	}

	getAutoSaveEnabled(): boolean {
		return this.config.get("autoSave", true);
	}

	getShowParameterHints(): boolean {
		return this.config.get("showParameterHints", true);
	}
}
