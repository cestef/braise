import * as vscode from "vscode";

export class BraiseFoldingProvider implements vscode.FoldingRangeProvider {
	provideFoldingRanges(document: vscode.TextDocument): vscode.FoldingRange[] {
		const foldingRanges: vscode.FoldingRange[] = [];
		const text = document.getText();
		const lines = text.split("\n");
		const stack: number[] = [];

		for (let i = 0; i < lines.length; i++) {
			const line = lines[i];

			let braceCount = 0;
			for (const char of line) {
				if (char === "{") {
					braceCount++;
				} else if (char === "}") {
					braceCount--;
				}
			}

			if (braceCount > 0) {
				for (let j = 0; j < braceCount; j++) {
					stack.push(i);
				}
			}

			if (braceCount < 0) {
				for (let j = 0; j < Math.abs(braceCount); j++) {
					const start = stack.pop();
					if (start !== undefined && i > start) {
						foldingRanges.push(new vscode.FoldingRange(start, i));
					}
				}
			}
		}

		return foldingRanges;
	}
}
