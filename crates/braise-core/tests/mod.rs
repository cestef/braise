#[cfg(test)]
mod tests {
    use braise_core::*;

    #[test]
    fn test_span_operations() {
        let file_id = FileId(1);
        let start_pos = Position::new(1, 5, 10);
        let end_pos = Position::new(2, 3, 25);
        let span = Span::new(start_pos, end_pos, file_id);

        assert_eq!(span.len(), 15); // 25 - 10
        assert!(!span.is_empty());

        let empty_span = Span::new(start_pos, start_pos, file_id);
        assert!(empty_span.is_empty());
        assert_eq!(empty_span.len(), 0);
    }

    #[test]
    fn test_span_merge() {
        let file_id = FileId(1);
        let pos1 = Position::new(1, 1, 0);
        let pos2 = Position::new(1, 5, 4);
        let pos3 = Position::new(2, 3, 15);

        let span1 = Span::new(pos1, pos2, file_id);
        let span2 = Span::new(pos2, pos3, file_id);
        let merged = span1.merge(&span2);

        assert_eq!(merged.start, pos1);
        assert_eq!(merged.end, pos3);
        assert_eq!(merged.file_id, file_id);
    }

    #[test]
    fn test_span_contains() {
        let file_id = FileId(1);
        let outer_start = Position::new(1, 1, 0);
        let outer_end = Position::new(3, 10, 50);
        let outer_span = Span::new(outer_start, outer_end, file_id);

        let inner_start = Position::new(2, 1, 10);
        let inner_end = Position::new(2, 20, 30);
        let inner_span = Span::new(inner_start, inner_end, file_id);

        assert!(outer_span.contains(&inner_span));
        assert!(!inner_span.contains(&outer_span));
    }

    #[test]
    fn test_spanned_map() {
        let file_id = FileId(1);
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4), file_id);

        let spanned_int = Spanned::new(42, span.clone());
        let spanned_string = spanned_int.map(|i| i.to_string());

        assert_eq!(spanned_string.value, "42");
        assert_eq!(spanned_string.span, span);
    }

    #[test]
    fn test_source_map() {
        let source = "line1\nline2\nline3".to_string();
        let file_id = FileId(1);
        let source_map = SourceMap::new(source, file_id);

        // Test position at start
        let pos_start = source_map.position_at_offset(0);
        assert_eq!(pos_start.line, 1);
        assert_eq!(pos_start.column, 1);

        // Test position at start of second line
        let pos_line2 = source_map.position_at_offset(6); // After "line1\n"
        assert_eq!(pos_line2.line, 2);
        assert_eq!(pos_line2.column, 1);

        // Test position in middle of second line
        let pos_middle = source_map.position_at_offset(8); // "li" in "line2"
        assert_eq!(pos_middle.line, 2);
        assert_eq!(pos_middle.column, 3);
    }

    #[test]
    fn test_source_map_span_creation() {
        let source = "hello\nworld".to_string();
        let file_id = FileId(1);
        let source_map = SourceMap::new(source, file_id);

        let span = source_map.span_from_range(6, 11); // "world"
        assert_eq!(span.start.line, 2);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.end.line, 2);
        assert_eq!(span.end.column, 6);
    }

    #[test]
    fn test_param_type_compatibility() {
        // Basic types
        assert!(ParamType::String.is_compatible_with(&ParamType::String));
        assert!(ParamType::Number.is_compatible_with(&ParamType::String)); // number -> string
        assert!(ParamType::Bool.is_compatible_with(&ParamType::String)); // bool -> string
        assert!(!ParamType::String.is_compatible_with(&ParamType::Number));

        // Array types
        let string_array = ParamType::Array(Box::new(ParamType::String));
        let number_array = ParamType::Array(Box::new(ParamType::Number));
        assert!(string_array.is_compatible_with(&string_array));
        assert!(!string_array.is_compatible_with(&number_array));

        // Enum types
        let enum1 = ParamType::Enum(vec!["a".to_string(), "b".to_string()]);
        let enum2 = ParamType::Enum(vec!["a".to_string(), "b".to_string()]);
        let enum3 = ParamType::Enum(vec!["c".to_string(), "d".to_string()]);
        assert!(enum1.is_compatible_with(&enum2));
        assert!(!enum1.is_compatible_with(&enum3));
    }

    #[test]
    fn test_param_type_default_values() {
        assert!(matches!(
            ParamType::String.default_value(),
            Expression::String(s) if s.is_empty()
        ));

        assert!(matches!(
            ParamType::Number.default_value(),
            Expression::Number(n) if n == 0.0
        ));

        assert!(matches!(
            ParamType::Bool.default_value(),
            Expression::Bool(false)
        ));

        assert!(matches!(
            ParamType::Array(Box::new(ParamType::String)).default_value(),
            Expression::Array(arr) if arr.is_empty()
        ));

        let enum_type = ParamType::Enum(vec!["first".to_string(), "second".to_string()]);
        assert!(matches!(
            enum_type.default_value(),
            Expression::String(s) if s == "first"
        ));
    }

    #[test]
    fn test_param_type_display() {
        assert_eq!(ParamType::String.to_string(), "string");
        assert_eq!(ParamType::Number.to_string(), "number");
        assert_eq!(ParamType::Bool.to_string(), "bool");

        let array_type = ParamType::Array(Box::new(ParamType::String));
        assert_eq!(array_type.to_string(), "array[string]");

        let enum_type = ParamType::Enum(vec!["dev".to_string(), "prod".to_string()]);
        assert_eq!(enum_type.to_string(), "enum[dev, prod]");

        assert_eq!(ParamType::Recipe.to_string(), "recipe");
    }

    #[test]
    fn test_symbol_table() {
        let mut table = SymbolTable::new();

        // Test scoping
        table.push_scope();

        let symbol1 = Symbol {
            name: "test".to_string(),
            kind: SymbolKind::Variable,
            span: Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4), FileId(1)),
            definition_span: Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4), FileId(1)),
        };

        assert!(table.define(symbol1.clone()).is_ok());
        assert!(table.lookup("test").is_some());

        // Test duplicate definition in same scope
        assert!(table.define(symbol1).is_err());

        // Test inner scope
        table.push_scope();
        let symbol2 = Symbol {
            name: "inner".to_string(),
            kind: SymbolKind::Variable,
            span: Span::new(Position::new(2, 1, 10), Position::new(2, 6, 15), FileId(1)),
            definition_span: Span::new(Position::new(2, 1, 10), Position::new(2, 6, 15), FileId(1)),
        };

        assert!(table.define(symbol2).is_ok());
        assert!(table.lookup("inner").is_some());
        assert!(table.lookup("test").is_some()); // Should find in outer scope

        // Pop inner scope
        table.pop_scope();
        assert!(table.lookup("inner").is_none());
        assert!(table.lookup("test").is_some());
    }

    #[test]
    fn test_find_first_existing_file() {
        use std::fs::File;

        // Create a temporary file
        let temp_file = "test_braise_file.tmp";
        File::create(temp_file).unwrap();

        // Test with existing file
        let files = &["nonexistent1.braise", temp_file, "nonexistent2.braise"];
        let result = find_first_existing_file(files);
        assert_eq!(result, Some(temp_file.to_string()));

        // Clean up
        std::fs::remove_file(temp_file).unwrap();

        // Test with no existing files
        let files = &["nonexistent1.braise", "nonexistent2.braise"];
        let result = find_first_existing_file(files);
        assert_eq!(result, None);
    }
}
