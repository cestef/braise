// grammar.js - Fixed Tree-sitter grammar for Braise DSL
module.exports = grammar({
	name: "braise",

	extras: ($) => [/\s/, $.comment],

	rules: {
		source_file: ($) => repeat($.recipe_definition),

		recipe_definition: ($) =>
			seq(
				"recipe",
				field("name", $.string_literal),
				optional($.dependencies),
				field("body", $.block)
			),

		dependencies: ($) => seq("->", "[", commaSep($.string_literal), "]"),

		block: ($) => seq("{", repeat($._statement), "}"),

		_statement: ($) =>
			choice(
				$.param_statement,
				$.if_statement,
				$.match_statement,
				$.for_statement,
				$.run_statement,
				$.exit_statement
			),

		param_statement: ($) =>
			seq(
				"param",
				field("name", $.identifier),
				":",
				field("type", $._type),
				optional(seq("=", field("default", $._expression)))
			),

		_type: ($) => choice($.primitive_type, $.array_type, $.union_type),

		primitive_type: ($) => choice("bool", "int", "string"),

		array_type: ($) => seq("[", $._type, "]"),

		union_type: ($) => seq("[", commaSep($.string_literal), "]"),

		if_statement: ($) =>
			seq(
				"if",
				field("condition", $._expression),
				field("then", $.block),
				repeat(
					seq("else", "if", field("condition", $._expression), field("then", $.block))
				),
				optional(seq("else", field("else", $.block)))
			),

		match_statement: ($) =>
			seq("match", field("expression", $._expression), "{", repeat($.match_arm), "}"),

		match_arm: ($) =>
			seq(
				field("pattern", choice($.string_literal, "_")),
				"=>",
				field("body", choice($.block, $._statement))
			),

		for_statement: ($) =>
			seq(
				"for",
				field("variable", $.identifier),
				"in",
				field("iterable", $._expression),
				field("body", $.block),
				optional("async")
			),

		run_statement: ($) => seq("run", field("command", $._expression)),

		exit_statement: ($) => seq("exit", field("code", $._expression)),

		_expression: ($) =>
			choice(
				$.binary_expression,
				$.unary_expression,
				$.call_expression,
				$.member_expression,
				$.conditional_expression,
				$.parenthesized_expression,
				$.interpolated_string,
				$.string_literal,
				$.identifier,
				$.number,
				$.boolean,
				$.array
			),

		// Add conditional expression back but with higher precedence
		conditional_expression: ($) =>
			prec(
				10,
				seq(
					"if",
					field("condition", $._expression),
					"{",
					field("then", $._expression),
					"}",
					"else",
					"{",
					field("else", $._expression),
					"}"
				)
			),

		binary_expression: ($) =>
			choice(
				prec.left(5, seq($._expression, "&&", $._expression)),
				prec.left(4, seq($._expression, "||", $._expression)),
				prec.left(7, seq($._expression, "==", $._expression)),
				prec.left(7, seq($._expression, "!=", $._expression)),
				prec.left(6, seq($._expression, "<", $._expression)),
				prec.left(6, seq($._expression, ">", $._expression)),
				prec.left(6, seq($._expression, "<=", $._expression)),
				prec.left(6, seq($._expression, ">=", $._expression))
			),

		unary_expression: ($) =>
			choice(prec(8, seq("!", $._expression)), prec(8, seq("-", $._expression))),

		call_expression: ($) =>
			prec(
				9,
				seq(field("function", $._expression), "(", optional(commaSep($._expression)), ")")
			),

		member_expression: ($) =>
			prec.left(
				11,
				seq(field("object", $._expression), ".", field("property", $.identifier))
			),

		parenthesized_expression: ($) => seq("(", $._expression, ")"),

		// Simple string literal (no interpolation)
		string_literal: ($) =>
			choice(
				seq('"', optional(alias(/[^"$]*/, $.string_content)), '"'),
				seq("'", optional(alias(/[^']*/, $.string_content)), "'")
			),

		// Interpolated string (contains ${...})
		interpolated_string: ($) =>
			seq(
				'"',
				repeat1(
					choice(alias(token.immediate(/[^"$]+/), $.string_content), $.interpolation)
				),
				'"'
			),

		interpolation: ($) => seq("${", $._expression, "}"),

		array: ($) => seq("[", optional(commaSep($._expression)), "]"),

		number: ($) => /\d+/,

		boolean: ($) => choice("true", "false"),

		identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

		comment: ($) => token(seq("//", /.*/)),
	},
});

function commaSep(rule) {
	return optional(seq(rule, repeat(seq(",", rule))));
}
