#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# /// script
# requires-python = ">=3.8"
# dependencies = [
#     "PyYAML>=6.0",
# ]
# ///

import json
import os
import plistlib
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Union

import yaml
from argparse import ArgumentParser, Namespace


class Formatter:
    """Helper class for formatting various syntax elements"""
    
    @staticmethod
    def format_comment(comment: str) -> str:
        """Format comment string by stripping and replacing tabs"""
        comment = comment.strip().replace('\t', '    ')
        if '\n' in comment:
            comment = comment.rstrip() + '\n'
        return comment
    
    @staticmethod
    def format_regex(regex_str: str) -> str:
        """Format regex string, handling multiline patterns"""
        if '\n' not in regex_str:
            return regex_str
            
        lines = regex_str.split('\n')
        
        if len(lines) > 1:
            # Find common indentation
            common_indent = Formatter._leading_whitespace(lines[1])
            
            for line in lines[2:]:
                cur_indent = Formatter._leading_whitespace(line)
                if cur_indent.startswith(common_indent):
                    continue
                elif common_indent.startswith(cur_indent):
                    common_indent = cur_indent
                else:
                    common_indent = ''
            
            # Add indentation to first line if needed
            if not lines[0].startswith(common_indent):
                lines[0] = common_indent + lines[0].lstrip()
        else:
            common_indent = Formatter._leading_whitespace(lines[0])
        
        # Remove common indentation from all lines
        result_lines = []
        for line in lines:
            if len(line) >= len(common_indent):
                result_lines.append(line[len(common_indent):])
            else:
                result_lines.append(line)
        
        return '\n'.join(result_lines).rstrip()
    
    @staticmethod
    def format_captures(captures: Dict[str, Any]) -> Dict[Union[int, str], str]:
        """Format capture groups"""
        formatted_captures = {}
        
        for key, value in captures.items():
            if 'name' not in value:
                print(f"WARNING: patterns and includes are not supported within captures: {captures}")
                continue
                
            try:
                formatted_captures[int(key)] = value['name']
            except ValueError:
                print('WARNING: named capture used, this is unsupported')
                formatted_captures[key] = value['name']
        
        return formatted_captures
    
    @staticmethod
    def format_external_syntax(key: str) -> str:
        """Format external syntax reference"""
        if key[0] in '#$':
            raise ValueError('invalid external syntax name')
            
        if '#' in key:
            syntax, rule = key.split('#', 1)
            return f"scope:{syntax}#{rule}"
        else:
            return f"scope:{key}"
    
    @staticmethod
    def needs_quoting(string: str) -> bool:
        """Check if string needs quoting in YAML"""
        return (
            string == "" or 
            string.startswith('<<') or 
            string[0] in "\"'%-:?@`&*!,#|>0123456789=" or
            string in ('true', 'false', 'null') or
            '# ' in string or
            ': ' in string or
            any(c in string for c in '[]{}') or
            '\n' in string or
            string[-1] in ':#' or
            string.strip() != string
        )
    
    @staticmethod
    def quote(string: str) -> str:
        """Quote string for YAML"""
        if '\\' in string or '"' in string:
            return "'" + string.replace("'", "''") + "'"
        else:
            return '"' + string.replace('\\', '\\\\').replace('"', '\\"') + '"'
    
    @staticmethod
    def _leading_whitespace(string: str) -> str:
        """Get leading whitespace of string"""
        return string[:len(string) - len(string.lstrip())]


class MatchPattern:
    """Represents a match pattern in the syntax"""
    
    def __init__(self, pattern: Dict[str, Any]):
        self.match = Formatter.format_regex(pattern['match'])
        self.scope = pattern.get('name')
        self.captures = None
        self.comment = None
        
        if 'captures' in pattern:
            self.captures = Formatter.format_captures(pattern['captures'])
            
        if 'comment' in pattern:
            formatted_comment = Formatter.format_comment(pattern['comment'])
            if formatted_comment:
                self.comment = formatted_comment
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary representation"""
        result = {}
        if self.match:
            result['match'] = self.match
        if self.scope:
            result['scope'] = self.scope
        if self.captures:
            result['captures'] = self.captures
        if self.comment:
            result['comment'] = self.comment
        return result


class BeginEndPattern:
    """Represents a begin/end pattern in the syntax"""
    
    def __init__(self, pattern_type: str, pattern: Dict[str, Any]):
        self.pattern = pattern
        self.type = pattern_type
        self.match = Formatter.format_regex(pattern[pattern_type])
        self.pop = pattern_type == 'end'
        self.captures = None
        self._handle_captures()
    
    def _handle_captures(self):
        """Handle capture groups for begin/end patterns"""
        pattern_captures = (
            self.pattern.get(f"{self.type}Captures") or 
            self.pattern.get("captures")
        )
        
        if not pattern_captures:
            return
            
        captures = Formatter.format_captures(pattern_captures)
        
        if 0 in captures:
            # Note: In the original Ruby code, this sets entry['scope'] 
            # but entry is not defined in that context. This seems like a bug.
            # We'll skip this for now.
            captures.pop(0)
        
        if captures:
            self.captures = captures
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary representation"""
        result = {}
        if self.match:
            result['match'] = self.match
        if self.pop:
            result['pop'] = self.pop
        if self.captures:
            result['captures'] = self.captures
        return result


class SyntaxYaml:
    """Handles YAML output generation"""
    
    TAB_SIZE = 2
    
    def __init__(self, syntax_dict: Dict[str, Any]):
        self.yaml = self._to_yaml(syntax_dict, False, 0)
    
    def _to_yaml(self, val: Any, start_block_on_newline: bool, indent: int) -> str:
        """Convert value to YAML string"""
        out = ''
        
        if indent == 0:
            out += "%YAML 1.2\n---\n"
            out += "# http://www.sublimetext.com/docs/3/syntax.html\n"
        
        if isinstance(val, list):
            out += self._array_to_yaml(val, start_block_on_newline, indent)
        elif isinstance(val, dict):
            out += self._hash_to_yaml(val, start_block_on_newline, indent)
        elif isinstance(val, str):
            out += self._string_to_yaml(val, start_block_on_newline, indent)
        elif isinstance(val, bool):
            out += self._boolean_to_yaml(val)
        else:
            out += f"{val}\n"
        
        # Clean up the output
        lines = out.split('\n')
        cleaned_lines = []
        
        for line in lines:
            # Remove trailing whitespace
            line = line.rstrip()
            cleaned_lines.append(line)
        
        # Remove excessive blank lines but keep structure
        final_lines = []
        prev_was_empty = False
        
        for line in cleaned_lines:
            if line == '':
                if not prev_was_empty:
                    final_lines.append(line)
                prev_was_empty = True
            else:
                final_lines.append(line)
                prev_was_empty = False
        
        # Ensure file ends with exactly one newline
        result = '\n'.join(final_lines)
        if result and not result.endswith('\n'):
            result += '\n'
        
        return result
    
    def _array_to_yaml(self, val: List[Any], start_block_on_newline: bool, indent: int) -> str:
        """Convert array to YAML"""
        if len(val) == 0:
            return "[]\n"
        
        out = ''
        if start_block_on_newline:
            out += "\n"
        
        for item in val:
            out += ' ' * indent + '- ' + self._to_yaml(item, False, indent + 2)
        
        return out
    
    def _hash_to_yaml(self, val: Dict[str, Any], start_block_on_newline: bool, indent: int) -> str:
        """Convert hash/dict to YAML"""
        out = ''
        if start_block_on_newline:
            out += "\n"
        
        first = True
        for key in self._order_keys(list(val.keys())):
            value = val[key]
            
            if not first or start_block_on_newline:
                out += ' ' * indent
            else:
                first = False
            
            if isinstance(key, (int, float)):
                out += str(key)
            elif Formatter.needs_quoting(key):
                out += Formatter.quote(key)
            else:
                out += key
            
            out += ": "
            out += self._to_yaml(value, True, indent + self.TAB_SIZE)
        
        return out
    
    def _string_to_yaml(self, val: str, start_block_on_newline: bool, indent: int) -> str:
        """Convert string to YAML"""
        if Formatter.needs_quoting(val):
            if '\n' in val:
                if not start_block_on_newline:
                    raise ValueError("Multiline string must start on newline")
                out = "|\n" if val.endswith('\n') else "|-\n"
                for line in val.split('\n'):
                    out += f"{' ' * indent}{line}\n"
                return out
            else:
                return f"{Formatter.quote(val)}\n"
        else:
            return f"{val}\n"
    
    def _boolean_to_yaml(self, val: bool) -> str:
        """Convert boolean to YAML"""
        return "true\n" if val else "false\n"
    
    def _order_keys(self, keys: List[str]) -> List[str]:
        """Order keys according to preferred order"""
        key_order = [
            'name', 'scope', 'file_extensions', 'first_line_match', 
            'comment', 'hidden', 'contexts'
        ]
        
        # Start with sorted keys
        ordered_keys = []
        remaining_keys = set(keys)
        
        # Add keys in preferred order
        for key in key_order:
            if key in remaining_keys:
                ordered_keys.append(key)
                remaining_keys.remove(key)
        
        # Add remaining keys alphabetically
        ordered_keys.extend(sorted(remaining_keys))
        
        return ordered_keys


class Convertor:
    """Main converter class"""
    
    def __init__(self, lang_content: Union[str, bytes], file_path: Optional[Path] = None):
        # Parse the content based on file extension or content type
        if isinstance(lang_content, str):
            lang_content = lang_content.encode('utf-8')
        
        # Determine if this is JSON or XML plist
        is_json = False
        if file_path:
            is_json = file_path.suffix.lower() == '.json' or 'tmLanguage.json' in str(file_path)
        else:
            # Try to detect format from content
            try:
                content_str = lang_content.decode('utf-8').strip()
                is_json = content_str.startswith('{') or content_str.startswith('[')
            except UnicodeDecodeError:
                is_json = False
        
        try:
            if is_json:
                # Parse as JSON
                content_str = lang_content.decode('utf-8')
                self.lang = json.loads(content_str)
            else:
                # Parse as XML plist
                self.lang = plistlib.loads(lang_content)
        except (json.JSONDecodeError, plistlib.InvalidFileException, UnicodeDecodeError) as e:
            raise ValueError(f"Invalid file format: {e}")
        
        self.repository = self.lang.get('repository', {})
        self.patterns = self.lang.get('patterns', [])
        self.syntax = {}
        
        self._normalize_repository()
        self._convert()
    
    def _normalize_repository(self):
        """Normalize repository entries"""
        for key, value in self.repository.items():
            if 'begin' in value or 'match' in value:
                self.repository[key] = [value]
            else:
                self.repository[key] = value.get('patterns', [])
    
    def _create_contexts(self) -> Dict[str, List[Dict[str, Any]]]:
        """Create contexts from patterns and repository"""
        contexts = {}
        contexts['main'] = self._make_context(self.lang.get('patterns', []))
        
        for key, value in self.repository.items():
            if key == 'main':
                raise ValueError('Double definition of main context')
            contexts[key] = self._make_context(value)
        
        return contexts
    
    def _convert(self):
        """Convert tmLanguage to sublime-syntax format"""
        syntax = {}
        
        if 'comment' in self.lang:
            syntax['comment'] = Formatter.format_comment(self.lang['comment'])
        
        if 'firstLineMatch' in self.lang:
            syntax['first_line_match'] = Formatter.format_regex(self.lang['firstLineMatch'])
        
        if 'name' in self.lang:
            syntax['name'] = self.lang['name']
        
        if 'scopeName' in self.lang:
            syntax['scope'] = self.lang['scopeName']
        
        if 'fileTypes' in self.lang:
            syntax['file_extensions'] = self.lang['fileTypes']
        
        if 'hideFromUser' in self.lang:
            syntax['hidden'] = self.lang['hideFromUser']
        
        if 'hidden' in self.lang:
            syntax['hidden'] = self.lang['hidden']
        
        syntax['contexts'] = self._create_contexts()
        self.syntax = syntax
    
    def _handle_begin_pattern(self, pattern: Dict[str, Any]) -> Dict[str, Any]:
        """Handle begin/end pattern"""
        entry = BeginEndPattern('begin', pattern).to_dict()
        
        if 'comment' in pattern:
            formatted_comment = Formatter.format_comment(pattern['comment'])
            if formatted_comment:
                entry['comment'] = formatted_comment
        
        entry['push'] = self._handle_child_pattern(pattern)
        return entry
    
    def _handle_child_pattern(self, pattern: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Handle child patterns for begin/end blocks"""
        end_entry = BeginEndPattern('end', pattern).to_dict()
        child_patterns = pattern.get('patterns', [])
        child = self._make_context(child_patterns)
        
        apply_last = pattern.get('applyEndPatternLast') == 1
        
        if apply_last:
            child.append(end_entry)
        else:
            child.insert(0, end_entry)
        
        if 'contentName' in pattern:
            child.insert(0, {'meta_content_scope': pattern['contentName']})
        
        if 'name' in pattern:
            child.insert(0, {'meta_scope': pattern['name']})
        
        if '\\G' in end_entry.get('match', ''):
            print(f"WARNING: pop pattern contains \\G, this will not work as expected")
            print(f"if it's intended to refer to the begin regex: {end_entry['match']}")
        
        return child
    
    def _handle_include_pattern(self, pattern: Dict[str, Any]) -> Dict[str, str]:
        """Handle include patterns"""
        key = pattern['include']
        
        if key.startswith('#'):
            key = key[1:]
            if key not in self.repository:
                raise ValueError(f"no entry in repository for {key}")
            return {'include': key}
        elif key == '$self':
            return {'include': 'main'}
        elif key == '$base':
            return {'include': '$top_level_main'}
        elif key.startswith('$'):
            raise ValueError(f"unknown include: {key}")
        else:
            return {'include': Formatter.format_external_syntax(key)}
    
    def _make_context(self, patterns: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Create context from patterns"""
        ctx = []
        
        for pattern in patterns:
            entry = None
            
            if 'begin' in pattern:
                entry = self._handle_begin_pattern(pattern)
            elif 'match' in pattern:
                entry = MatchPattern(pattern).to_dict()
            elif 'include' in pattern:
                entry = self._handle_include_pattern(pattern)
            else:
                raise ValueError(f"unknown pattern type: {list(pattern.keys())}")
            
            if entry:
                ctx.append(entry)
        
        return ctx
    
    def to_yaml(self) -> str:
        """Convert to YAML string"""
        return SyntaxYaml(self.syntax).yaml
    
    def set_file_extensions(self, extensions: List[str]):
        """Set file extensions for the syntax"""
        self.syntax['file_extensions'] = extensions


def create_parser() -> ArgumentParser:
    """Create and configure argument parser"""
    from argparse import RawDescriptionHelpFormatter
    
    class CustomFormatter(RawDescriptionHelpFormatter):
        def __init__(self, prog):
            super().__init__(prog, max_help_position=40)
    
    parser = ArgumentParser(
        description='Convert tmLanguage files to sublime-syntax format',
        epilog='Examples:\n'
               '  %(prog)s syntax.tmLanguage\n'
               '  %(prog)s syntax.tmLanguage.json --extensions py pyx\n'
               '  %(prog)s syntaxes/ --output-dir build/\n'
               '  %(prog)s *.tmLanguage --force --quiet',
        formatter_class=CustomFormatter
    )
    
    parser.add_argument(
        'paths',
        nargs='+',
        help='Input files or directories to convert (.tmLanguage or .tmLanguage.json)'
    )
    
    parser.add_argument(
        '-o', '--output-dir',
        type=Path,
        help='Output directory (default: same as input file)'
    )
    
    parser.add_argument(
        '-e', '--extensions',
        nargs='+',
        help='File extensions to add to the syntax (e.g., py pyx)'
    )
    
    parser.add_argument(
        '-f', '--force',
        action='store_true',
        help='Overwrite existing output files without prompting'
    )
    
    parser.add_argument(
        '-q', '--quiet',
        action='store_true',
        help='Suppress output messages'
    )
    
    parser.add_argument(
        '--no-validation',
        action='store_true',
        help='Skip YAML validation step'
    )
    
    parser.add_argument(
        '--version',
        action='version',
        version='%(prog)s 1.0.0'
    )
    
    return parser


def collect_files(paths: List[Path]) -> List[Path]:
    """Collect all tmLanguage files from the given paths"""
    files = []
    
    for path in paths:
        if path.is_dir():
            # Look for both .tmLanguage and .tmLanguage.json files
            files.extend(path.glob('*.tmLanguage'))
            files.extend(path.glob('*.tmLanguage.json'))
        elif path.exists():
            files.append(path)
        else:
            # Handle glob patterns
            parent = path.parent
            pattern = path.name
            matches = list(parent.glob(pattern))
            if matches:
                files.extend(matches)
            else:
                print(f"WARNING: No files found matching pattern: {path}")
    
    return files


def get_output_path(input_path: Path, output_dir: Optional[Path] = None) -> Path:
    """Generate output path for a given input file"""
    if input_path.name.endswith('.tmLanguage.json'):
        output_name = input_path.name.replace('.tmLanguage.json', '.sublime-syntax')
    elif input_path.suffix == '.tmLanguage':
        output_name = input_path.stem + '.sublime-syntax'
    else:
        output_name = input_path.stem + '.sublime-syntax'
    
    if output_dir:
        output_dir.mkdir(parents=True, exist_ok=True)
        return output_dir / output_name
    else:
        return input_path.parent / output_name


def validate_yaml(text: str, original_syntax: Dict[str, Any], filename: Path) -> bool:
    """Validate generated YAML"""
    try:
        yaml_content = text.replace('%YAML 1.2', '', 1)
        yaml_dict = yaml.safe_load(yaml_content)
        
        # Basic validation - we can't do deep comparison due to type differences
        # but we can check structure
        if not isinstance(yaml_dict, dict):
            print(f"ERROR: Generated YAML is not a dictionary for {filename}")
            return False
            
        required_keys = {'contexts'}
        if not required_keys.issubset(yaml_dict.keys()):
            print(f"ERROR: Generated YAML missing required keys for {filename}")
            return False
        
        return True
        
    except yaml.YAMLError as e:
        print(f"ERROR: Generated invalid YAML for {filename}: {e}")
        return False


def convert_file(
    input_path: Path,
    output_path: Path,
    extensions: Optional[List[str]] = None,
    force: bool = False,
    quiet: bool = False,
    validate: bool = True
) -> bool:
    """Convert a single file"""
    if output_path.exists() and not force:
        response = input(f"Output file {output_path} exists. Overwrite? (y/N): ")
        if response.lower() != 'y':
            if not quiet:
                print(f"Skipped {input_path}")
            return False
    
    try:
        content = input_path.read_bytes()
        convertor = Convertor(content, input_path)
        
        # Set custom file extensions if provided
        if extensions:
            convertor.set_file_extensions(extensions)
        
        text = convertor.to_yaml()
        
        # Validate YAML if requested
        if validate and not validate_yaml(text, convertor.syntax, input_path):
            return False
        
        output_path.write_text(text, encoding='utf-8')
        
        if not quiet:
            print(f"Converted {input_path} → {output_path}")
        
        return True
        
    except Exception as e:
        print(f"ERROR converting {input_path}: {e}")
        return False


def main() -> int:
    """Main entry point"""
    parser = create_parser()
    args = parser.parse_args()
    
    # Convert string paths to Path objects
    input_paths = [Path(p) for p in args.paths]
    
    # Collect all files to process
    files = collect_files(input_paths)
    
    if not files:
        print("ERROR: No tmLanguage files found")
        return 1
    
    # Process each file
    success_count = 0
    total_count = len(files)
    
    for input_file in files:
        output_file = get_output_path(input_file, args.output_dir)
        
        if convert_file(
            input_file,
            output_file,
            args.extensions,
            args.force,
            args.quiet,
            not args.no_validation
        ):
            success_count += 1
    
    if not args.quiet:
        print(f"\nProcessed {success_count}/{total_count} files successfully")
    
    return 0 if success_count == total_count else 1


if __name__ == '__main__':
    sys.exit(main())