export interface BraiseRecipe {
	name: string;
	parameters: Array<{
		name: string;
		type: string;
		defaultValue?: string;
	}>;
	dependencies: string[];
	lineNumber: number;
}

export interface ValidationResult {
	isValid: boolean;
	errors: string[];
	recipeCount: number;
}

export class BraiseParser {
	parseRecipes(text: string): BraiseRecipe[] {
		const recipes: BraiseRecipe[] = [];
		const lines = text.split("\n");

		for (let i = 0; i < lines.length; i++) {
			const line = lines[i].trim();
			const recipeMatch = line.match(
				/recipe\s+"([^"]+)"(?:\s*->\s*\[([^\]]+)\])?\s*\{/,
			);

			if (recipeMatch) {
				const name = recipeMatch[1];
				const dependencies = recipeMatch[2]
					? recipeMatch[2].split(",").map((dep) => dep.trim().replace(/"/g, ""))
					: [];

				const parameters = this.extractParameters(text, i);

				recipes.push({
					name,
					parameters,
					dependencies,
					lineNumber: i,
				});
			}
		}

		return recipes;
	}

	private extractParameters(
		text: string,
		recipeStartLine: number,
	): Array<{ name: string; type: string; defaultValue?: string }> {
		const lines = text.split("\n");
		const parameters: Array<{
			name: string;
			type: string;
			defaultValue?: string;
		}> = [];

		// Look for parameters in the lines following the recipe declaration
		for (let i = recipeStartLine + 1; i < lines.length; i++) {
			const line = lines[i].trim();

			// Stop if we hit a closing brace or another recipe
			if (line.includes("}") || line.startsWith("recipe ")) {
				break;
			}

			// Match parameter declarations: param name: type = default
			const paramMatch = line.match(
				/param\s+(\w+):\s*([^=]+?)(?:\s*=\s*(.+))?$/,
			);
			if (paramMatch) {
				const name = paramMatch[1];
				const type = paramMatch[2].trim();
				const defaultValue = paramMatch[3]?.trim();

				parameters.push({
					name,
					type,
					defaultValue,
				});
			}
		}

		return parameters;
	}

	validateFile(text: string): ValidationResult {
		const errors: string[] = [];
		const recipes = this.parseRecipes(text);

		// Check for duplicate recipe names
		const names = recipes.map((r) => r.name);
		const duplicates = names.filter(
			(name, index) => names.indexOf(name) !== index,
		);
		if (duplicates.length > 0) {
			errors.push(
				`Duplicate recipe names: ${[...new Set(duplicates)].join(", ")}`,
			);
		}

		// Check for missing dependencies
		for (const recipe of recipes) {
			for (const dep of recipe.dependencies) {
				if (!names.includes(dep)) {
					errors.push(
						`Recipe "${recipe.name}" depends on non-existent recipe "${dep}"`,
					);
				}
			}
		}

		return {
			isValid: errors.length === 0,
			errors,
			recipeCount: recipes.length,
		};
	}
}
