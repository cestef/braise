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

        let pos_start = source_map.position_at_offset(0);
        assert_eq!(pos_start.line, 1);
        assert_eq!(pos_start.column, 1);

        let pos_line2 = source_map.position_at_offset(6);
        assert_eq!(pos_line2.line, 2);
        assert_eq!(pos_line2.column, 1);

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
        assert!(BraiseType::String.is_compatible_with(&BraiseType::String));
        assert!(BraiseType::Number.is_compatible_with(&BraiseType::String));
        assert!(BraiseType::Bool.is_compatible_with(&BraiseType::String));
        assert!(!BraiseType::String.is_compatible_with(&BraiseType::Number));

        let string_array = BraiseType::Array(Box::new(BraiseType::String));
        let number_array = BraiseType::Array(Box::new(BraiseType::Number));
        assert!(string_array.is_compatible_with(&string_array));
        assert!(!string_array.is_compatible_with(&number_array));

        let enum1 = BraiseType::Enum(vec!["a".to_string(), "b".to_string()]);
        let enum2 = BraiseType::Enum(vec!["a".to_string(), "b".to_string()]);
        let enum3 = BraiseType::Enum(vec!["c".to_string(), "d".to_string()]);
        assert!(enum1.is_compatible_with(&enum2));
        assert!(!enum1.is_compatible_with(&enum3));
    }

    #[test]
    fn test_param_type_default_values() {
        assert!(matches!(
            BraiseType::String.default_value(),
            TypedValue { value, value_type } if matches!(&value, ValueData::String(s) if s == "") && value_type == BraiseType::String
        ));

        assert!(matches!(
            BraiseType::Number.default_value(),
            TypedValue { value, value_type } if matches!(value, ValueData::Number(n) if n == 0.0) && value_type == BraiseType::Number
        ));

        assert!(matches!(
            BraiseType::Bool.default_value(),
            TypedValue { value, value_type } if matches!(value, ValueData::Bool(b) if !b) && value_type == BraiseType::Bool
        ));

        assert!(matches!(
            BraiseType::Array(Box::new(BraiseType::String)).default_value(),
            TypedValue { value, value_type } if matches!(&value, ValueData::Array(arr) if arr.is_empty() && value_type == BraiseType::Array(Box::new(BraiseType::String)))
        ));

        let enum_type = BraiseType::Enum(vec!["first".to_string(), "second".to_string()]);
        assert!(matches!(
            enum_type.default_value(),
            TypedValue { value, value_type } if matches!(&value, ValueData::String(s) if s == "first") && value_type == BraiseType::Enum(vec!["first".to_string(), "second".to_string()])
        ));
    }

    #[test]
    fn test_param_type_display() {
        assert_eq!(BraiseType::String.to_string(), "string");
        assert_eq!(BraiseType::Number.to_string(), "number");
        assert_eq!(BraiseType::Bool.to_string(), "bool");

        let array_type = BraiseType::Array(Box::new(BraiseType::String));
        assert_eq!(array_type.to_string(), "[string]");

        let enum_type = BraiseType::Enum(vec!["dev".to_string(), "prod".to_string()]);
        assert_eq!(enum_type.to_string(), "{dev | prod}");

        assert_eq!(BraiseType::Recipe.to_string(), "recipe");
    }

    #[test]
    fn test_symbol_table() {
        let mut table = SymbolTable::new();

        table.push_scope();

        let symbol1 = Symbol {
            name: "test".to_string(),
            kind: SymbolKind::Variable,
            span: Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4), FileId(1)),
            definition_span: Span::new(Position::new(1, 1, 0), Position::new(1, 5, 4), FileId(1)),
        };

        assert!(table.define(symbol1.clone()).is_ok());
        assert!(table.lookup("test").is_some());

        assert!(table.define(symbol1).is_err());

        table.push_scope();
        let symbol2 = Symbol {
            name: "inner".to_string(),
            kind: SymbolKind::Variable,
            span: Span::new(Position::new(2, 1, 10), Position::new(2, 6, 15), FileId(1)),
            definition_span: Span::new(Position::new(2, 1, 10), Position::new(2, 6, 15), FileId(1)),
        };

        assert!(table.define(symbol2).is_ok());
        assert!(table.lookup("inner").is_some());
        assert!(table.lookup("test").is_some());

        table.pop_scope();
        assert!(table.lookup("inner").is_none());
        assert!(table.lookup("test").is_some());
    }
}
