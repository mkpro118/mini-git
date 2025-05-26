use std::fs;
use std::io::{self, BufRead};
use std::iter::Peekable;
use std::path::{Component, Path, PathBuf};
use std::str::Chars;

/// Represents the smallest matching units within a path segment pattern.
#[derive(Debug, Clone, PartialEq)]
enum Atom {
    /// Matches a literal string sequence.
    Literal(String),
    /// Matches `*` (zero or more characters, excluding '/').
    Asterisk,
    /// Matches `?` (any single character, excluding '/').
    QuestionMark,
    /// Matches a character class (e.g., `[abc]`, `[a-z]`, `[!0-9]`).
    CharClass { members: Vec<char>, inverted: bool },
}

/// Represents a token within a slash-separated gitignore pattern.
#[derive(Debug, Clone, PartialEq)]
enum PathSegmentPatternToken {
    /// A sequence of `Atom`s that must collectively match a single path component.
    Segment(Vec<Atom>),
    /// Represents `**` (double asterisk wildcard), matching zero or more directory levels.
    DoubleAsterisk,
}

/// Defines whether a pattern targets any path, or directories specifically.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PatternTargetType {
    FileOrDirectory,
    DirectoryOnly,
}

#[derive(Debug, PartialEq)]
pub enum GitIgnoreResult {
    NotIgnored,
    Ignored {
        ignore_file: PathBuf,
        line_number: usize,
        pattern: String,
    },
}

/// Represents a compiled gitignore line/rule.
#[derive(Debug, Clone)]
pub struct GitignoreRule {
    /// The original pattern string.
    pub raw_pattern: String,
    /// The line number in the .gitignore file.
    pub line_number: usize,
    /// The path to the .gitignore file.
    pub file_path: PathBuf,
    /// The tokenized pattern, e.g., `["src", "**", "*.js"]`.
    tokens: Vec<PathSegmentPatternToken>,
    /// True if the pattern is a negation (starts with '!').
    is_negation: bool,
    /// Specifies if the pattern targets only directories or any path.
    target_type: PatternTargetType,
    /// The absolute path to the directory containing the .gitignore file.
    base_path: PathBuf,
    /// True if the pattern (after '!') originally had no slashes and didn't start with one.
    /// Such patterns get an implicit `**/` prefix during matching. Excludes the `**` pattern itself.
    prepend_double_asterisk: bool,
}

/// Manages a set of gitignore rules and provides matching capabilities.
#[derive(Debug)]
pub struct GitignoreSet {
    rules: Vec<GitignoreRule>,
    repo_root: PathBuf,
}

impl GitignoreSet {
    pub fn new(repo_root: PathBuf) -> Self {
        GitignoreSet {
            rules: Vec::new(),
            repo_root,
        }
    }

    fn add_rule(&mut self, rule: GitignoreRule) {
        self.rules.push(rule);
    }

    pub fn add_patterns_from_file(
        &mut self,
        gitignore_file_path: &Path,
    ) -> Result<(), String> {
        let file = fs::File::open(gitignore_file_path).map_err(|_| {
            format!("Failed to open file {}", gitignore_file_path.display())
        })?;
        let reader = io::BufReader::new(file);

        let gitignore_dir = gitignore_file_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();

        for (idx, line_result) in reader.lines().enumerate() {
            let line_number = idx + 1;
            let line = line_result.map_err(|_| {
                format!(
                    "Failed to read line number {line_number} from '{}'",
                    gitignore_file_path.display()
                )
            })?;
            if let Some(rule) = parse_line_to_rule(
                &line,
                line_number,
                gitignore_file_path,
                &gitignore_dir,
            ) {
                self.rules.push(rule);
            }
        }
        Ok(())
    }

    pub fn is_ignored(
        &self,
        absolute_path_to_check: &Path,
        is_dir: bool,
    ) -> GitIgnoreResult {
        for rule in self.rules.iter().rev() {
            if Self::does_rule_match_path(rule, absolute_path_to_check, is_dir)
            {
                return if rule.is_negation {
                    GitIgnoreResult::NotIgnored
                } else {
                    GitIgnoreResult::Ignored {
                        ignore_file: rule.file_path.clone(),
                        line_number: rule.line_number,
                        pattern: rule.raw_pattern.clone(),
                    }
                };
            }
        }
        GitIgnoreResult::NotIgnored
    }

    fn does_rule_match_path(
        rule: &GitignoreRule,
        absolute_path_to_check: &Path,
        is_path_dir: bool,
    ) -> bool {
        let path_relative_to_rule_base: PathBuf;

        if let Ok(p) = absolute_path_to_check.strip_prefix(&rule.base_path) {
            path_relative_to_rule_base = p.to_path_buf();
        } else if absolute_path_to_check == rule.base_path {
            path_relative_to_rule_base = PathBuf::new();
        } else {
            return false;
        }

        let target_path_segments: Vec<&str> = path_relative_to_rule_base
            .components()
            .filter_map(|c| match c {
                Component::Normal(os_str) => os_str.to_str(),
                _ => None,
            })
            .collect();

        let mut final_pattern_tokens = rule.tokens.clone();

        if rule.prepend_double_asterisk
            && final_pattern_tokens.first()
                != Some(&PathSegmentPatternToken::DoubleAsterisk)
        {
            final_pattern_tokens
                .insert(0, PathSegmentPatternToken::DoubleAsterisk);
        }

        if !final_pattern_tokens.is_empty() {
            let mut deduped_tokens: Vec<PathSegmentPatternToken> = Vec::new();
            deduped_tokens.push(final_pattern_tokens[0].clone());
            for token in final_pattern_tokens.iter().skip(1) {
                if !(token == &PathSegmentPatternToken::DoubleAsterisk
                    && deduped_tokens.last()
                        == Some(&PathSegmentPatternToken::DoubleAsterisk))
                {
                    deduped_tokens.push(token.clone());
                }
            }
            final_pattern_tokens = deduped_tokens;
        }

        if Self::match_tokens_recursive(
            &final_pattern_tokens,
            &target_path_segments,
            0,
            0,
        ) {
            if rule.target_type == PatternTargetType::DirectoryOnly
                && !is_path_dir
            {
                return false;
            }
            if rule.tokens.is_empty() && target_path_segments.is_empty() {
                if rule.target_type == PatternTargetType::DirectoryOnly {
                    return is_path_dir;
                }
                return true;
            }
            if !final_pattern_tokens.is_empty() {
                return true;
            }
        }
        false
    }

    fn match_tokens_recursive(
        pattern_tokens: &[PathSegmentPatternToken],
        path_segments: &[&str],
        p_idx: usize,
        s_idx: usize,
    ) -> bool {
        if p_idx == pattern_tokens.len() {
            return s_idx == path_segments.len();
        }
        if s_idx == path_segments.len() {
            return pattern_tokens[p_idx..].iter().all(|token| {
                *token == PathSegmentPatternToken::DoubleAsterisk
            });
        }

        match &pattern_tokens[p_idx] {
            PathSegmentPatternToken::Segment(atoms) => {
                if Self::match_atoms_to_segment(
                    atoms,
                    path_segments[s_idx],
                    0,
                    0,
                ) {
                    return Self::match_tokens_recursive(
                        pattern_tokens,
                        path_segments,
                        p_idx + 1,
                        s_idx + 1,
                    );
                }
                false
            }
            PathSegmentPatternToken::DoubleAsterisk => {
                if Self::match_tokens_recursive(
                    pattern_tokens,
                    path_segments,
                    p_idx + 1,
                    s_idx,
                ) {
                    return true;
                }
                if s_idx < path_segments.len()
                    && Self::match_tokens_recursive(
                        pattern_tokens,
                        path_segments,
                        p_idx,
                        s_idx + 1,
                    )
                {
                    return true;
                }
                false
            }
        }
    }

    fn match_atoms_to_segment(
        atoms: &[Atom],
        segment_text: &str,
        a_idx: usize,
        char_idx: usize,
    ) -> bool {
        let text_chars: Vec<char> = segment_text.chars().collect();
        if a_idx == atoms.len() {
            return char_idx == text_chars.len();
        }

        match &atoms[a_idx] {
            Atom::Literal(s) => {
                let s_chars: Vec<char> = s.chars().collect();
                if char_idx + s_chars.len() <= text_chars.len()
                    && s_chars[..]
                        == text_chars[char_idx..char_idx + s_chars.len()]
                {
                    return Self::match_atoms_to_segment(
                        atoms,
                        segment_text,
                        a_idx + 1,
                        char_idx + s_chars.len(),
                    );
                }
                false
            }
            Atom::QuestionMark => {
                if char_idx < text_chars.len() {
                    return Self::match_atoms_to_segment(
                        atoms,
                        segment_text,
                        a_idx + 1,
                        char_idx + 1,
                    );
                }
                false
            }
            Atom::CharClass { members, inverted } => {
                if char_idx < text_chars.len() {
                    let current_char = text_chars[char_idx];
                    let matched = members.contains(&current_char);
                    if (matched && !*inverted) || (!matched && *inverted) {
                        return Self::match_atoms_to_segment(
                            atoms,
                            segment_text,
                            a_idx + 1,
                            char_idx + 1,
                        );
                    }
                }
                false
            }
            Atom::Asterisk => {
                if Self::match_atoms_to_segment(
                    atoms,
                    segment_text,
                    a_idx + 1,
                    char_idx,
                ) {
                    return true;
                }
                if char_idx < text_chars.len()
                    && Self::match_atoms_to_segment(
                        atoms,
                        segment_text,
                        a_idx,
                        char_idx + 1,
                    )
                {
                    return true;
                }
                false
            }
        }
    }
}

/// Internal struct used during parsing.
struct ParsedPatternInfo {
    core_pattern_for_tokenization: String,
    target_type: PatternTargetType,
    prepend_double_asterisk: bool,
}

fn preprocess_gitignore_line(line: &str) -> Option<String> {
    let mut current_pattern_str = line.trim_start();

    if current_pattern_str.starts_with("\\#") {
        current_pattern_str = &current_pattern_str[1..];
    } else if current_pattern_str.starts_with('#') {
        return None;
    }
    if current_pattern_str.is_empty() {
        return None;
    }

    let mut processed_pattern =
        String::with_capacity(current_pattern_str.len());
    let mut chars_iter = current_pattern_str.chars().peekable();
    let mut temp_trailing_spaces = String::new();

    while let Some(ch) = chars_iter.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars_iter.peek() {
                if next_ch == ' ' {
                    processed_pattern.push_str(&temp_trailing_spaces);
                    temp_trailing_spaces.clear();
                    processed_pattern.push(ch);
                    processed_pattern.push(chars_iter.next().unwrap());
                } else {
                    processed_pattern.push_str(&temp_trailing_spaces);
                    temp_trailing_spaces.clear();
                    processed_pattern.push(ch);
                }
            } else {
                processed_pattern.push_str(&temp_trailing_spaces);
                temp_trailing_spaces.clear();
                processed_pattern.push(ch);
                break;
            }
        } else if ch == ' ' {
            temp_trailing_spaces.push(' ');
        } else {
            processed_pattern.push_str(&temp_trailing_spaces);
            temp_trailing_spaces.clear();
            processed_pattern.push(ch);
        }
    }
    if processed_pattern.is_empty() && temp_trailing_spaces.is_empty() {
        return None;
    }
    Some(processed_pattern)
}

fn analyze_pattern_structure(
    pattern_str_after_negation_and_preprocessing: String,
) -> ParsedPatternInfo {
    let mut work_pattern = pattern_str_after_negation_and_preprocessing;

    let original_had_leading_slash = work_pattern.starts_with('/');
    let original_had_any_slashes = work_pattern.contains('/');
    let is_just_double_asterisk = work_pattern == "**";
    let prepend_double_asterisk = !original_had_leading_slash
        && !original_had_any_slashes
        && !is_just_double_asterisk;

    let mut target_type = PatternTargetType::FileOrDirectory;
    if work_pattern.ends_with('/') {
        if work_pattern.ends_with("\\/") {
            let len = work_pattern.len();
            work_pattern.replace_range(len - 2.., "/");
        } else {
            target_type = PatternTargetType::DirectoryOnly;
            if !work_pattern.is_empty() {
                work_pattern.pop();
            }
        }
    }

    if original_had_leading_slash && !work_pattern.is_empty() {
        work_pattern.remove(0);
    }

    ParsedPatternInfo {
        core_pattern_for_tokenization: work_pattern,
        target_type,
        prepend_double_asterisk,
    }
}

fn parse_line_to_rule(
    line: &str,
    line_number: usize,
    gitignore_file_path: &Path,
    gitignore_file_dir: &Path,
) -> Option<GitignoreRule> {
    let mut pattern_after_initial_preprocessing =
        preprocess_gitignore_line(line)?;

    let original_raw_pattern = line.to_string();

    let mut is_negation = false;
    if pattern_after_initial_preprocessing.starts_with('!') {
        if pattern_after_initial_preprocessing.len() > 1
            && !(pattern_after_initial_preprocessing.starts_with("\\!"))
        {
            is_negation = true;
            pattern_after_initial_preprocessing.remove(0);
        } else if pattern_after_initial_preprocessing.starts_with("\\!") {
            pattern_after_initial_preprocessing.remove(0);
        }
    }

    let structure_info =
        analyze_pattern_structure(pattern_after_initial_preprocessing.clone());

    let mut tokens: Vec<PathSegmentPatternToken> = Vec::new();
    if !structure_info.core_pattern_for_tokenization.is_empty() {
        let mut last_was_double_asterisk = false;
        for segment_str in
            structure_info.core_pattern_for_tokenization.split('/')
        {
            if segment_str == "**" {
                if !last_was_double_asterisk {
                    tokens.push(PathSegmentPatternToken::DoubleAsterisk);
                }
                last_was_double_asterisk = true;
            } else if !segment_str.is_empty() {
                tokens.push(PathSegmentPatternToken::Segment(
                    parse_segment_to_atoms(segment_str),
                ));
                last_was_double_asterisk = false;
            }
        }
    }

    Some(GitignoreRule {
        raw_pattern: original_raw_pattern,
        line_number,
        file_path: gitignore_file_path.to_path_buf(),
        tokens,
        is_negation,
        target_type: structure_info.target_type,
        base_path: gitignore_file_dir.to_path_buf(),
        prepend_double_asterisk: structure_info.prepend_double_asterisk,
    })
}

fn parse_char_class_atom_from_iterator(chars: &mut Peekable<Chars>) -> Atom {
    let mut members = Vec::new();
    let mut inverted = false;
    let mut in_escape_char_class = false;
    let mut is_first_char_in_class_content = true;
    let mut last_added_char_for_range: Option<char> = None;
    let mut just_processed_range = false;

    if chars.peek() == Some(&'!') {
        chars.next();
        inverted = true;
    }

    if chars.peek() == Some(&']') {
        let c = chars.next().unwrap();
        members.push(c);
        last_added_char_for_range = Some(c);
        is_first_char_in_class_content = false;
    }

    while let Some(class_char) = chars.next() {
        if in_escape_char_class {
            members.push(class_char);
            last_added_char_for_range = Some(class_char);
            just_processed_range = false;
            in_escape_char_class = false;
        } else {
            match class_char {
                '\\' => {
                    just_processed_range = false;
                    in_escape_char_class = true;
                }
                ']' => break,
                '-' => {
                    let dash_result = handle_dash_character(
                        chars,
                        is_first_char_in_class_content,
                        last_added_char_for_range,
                        just_processed_range,
                    );
                    members.extend(dash_result.chars_to_add);
                    last_added_char_for_range = dash_result.new_last_char;
                    just_processed_range = dash_result.just_processed_range;
                }
                _ => {
                    members.push(class_char);
                    last_added_char_for_range = Some(class_char);
                    just_processed_range = false;
                }
            }
        }
        is_first_char_in_class_content = false;
    }

    if in_escape_char_class {
        members.push('\\');
    }

    members.sort_unstable();
    members.dedup();
    Atom::CharClass { members, inverted }
}

struct DashHandlerResult {
    chars_to_add: Vec<char>,
    new_last_char: Option<char>,
    just_processed_range: bool,
}

fn handle_dash_character(
    chars: &mut Peekable<Chars>,
    is_first_char_in_class_content: bool,
    last_added_char_for_range: Option<char>,
    just_processed_range: bool,
) -> DashHandlerResult {
    if is_first_char_in_class_content
        || chars.peek() == Some(&']')
        || last_added_char_for_range.is_none()
        || last_added_char_for_range == Some('-')
        || just_processed_range
    {
        return DashHandlerResult {
            chars_to_add: vec!['-'],
            new_last_char: Some('-'),
            just_processed_range: false,
        };
    }

    if let Some(start_range_char) = last_added_char_for_range {
        process_character_range(chars, start_range_char)
    } else {
        DashHandlerResult {
            chars_to_add: vec!['-'],
            new_last_char: Some('-'),
            just_processed_range: false,
        }
    }
}

fn process_character_range(
    chars: &mut Peekable<Chars>,
    start_range_char: char,
) -> DashHandlerResult {
    if let Some(&next_char_peek) = chars.peek() {
        match next_char_peek {
            '\\' => process_escaped_range_end(chars, start_range_char),
            ']' => DashHandlerResult {
                chars_to_add: vec!['-'],
                new_last_char: Some('-'),
                just_processed_range: false,
            },
            _ => {
                let end_range_char = chars.next().unwrap();
                create_character_range(start_range_char, end_range_char)
            }
        }
    } else {
        DashHandlerResult {
            chars_to_add: vec!['-'],
            new_last_char: Some('-'),
            just_processed_range: false,
        }
    }
}

fn process_escaped_range_end(
    chars: &mut Peekable<Chars>,
    start_range_char: char,
) -> DashHandlerResult {
    chars.next(); // consume the '\'
    if let Some(escaped_range_end) = chars.next() {
        create_character_range(start_range_char, escaped_range_end)
    } else {
        DashHandlerResult {
            chars_to_add: vec!['-', '\\'],
            new_last_char: Some('\\'),
            just_processed_range: false,
        }
    }
}

fn create_character_range(
    start_char: char,
    end_char: char,
) -> DashHandlerResult {
    if start_char <= end_char {
        let mut chars_to_add = Vec::new();
        for c_val in (start_char as u32 + 1)..=(end_char as u32) {
            if let Some(c) = std::char::from_u32(c_val) {
                chars_to_add.push(c);
            }
        }
        DashHandlerResult {
            chars_to_add,
            new_last_char: Some(end_char),
            just_processed_range: true,
        }
    } else {
        DashHandlerResult {
            chars_to_add: vec!['-', end_char],
            new_last_char: Some(end_char),
            just_processed_range: false,
        }
    }
}

fn flush_literal(literal: &mut String, atoms_list: &mut Vec<Atom>) {
    if !literal.is_empty() {
        atoms_list.push(Atom::Literal(std::mem::take(literal)));
    }
}

fn parse_segment_to_atoms(segment_str: &str) -> Vec<Atom> {
    let mut atoms = Vec::new();
    let mut chars = segment_str.chars().peekable();
    let mut current_literal = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(escaped_char) = chars.next() {
                    current_literal.push(escaped_char);
                } else {
                    current_literal.push('\\');
                }
            }
            '*' => {
                flush_literal(&mut current_literal, &mut atoms);
                if atoms.last() != Some(&Atom::Asterisk) {
                    atoms.push(Atom::Asterisk);
                }
            }
            '?' => {
                flush_literal(&mut current_literal, &mut atoms);
                atoms.push(Atom::QuestionMark);
            }
            '[' => {
                flush_literal(&mut current_literal, &mut atoms);
                atoms.push(parse_char_class_atom_from_iterator(&mut chars));
            }
            _ => {
                current_literal.push(ch);
            }
        }
    }
    flush_literal(&mut current_literal, &mut atoms);
    atoms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rp(path_str: &str) -> PathBuf {
        if path_str.starts_with('/') {
            PathBuf::from(path_str)
        } else {
            PathBuf::from(format!("/{path_str}"))
        }
    }

    macro_rules! check_ignore_assert {
        ($repo_root_str:expr, $rules_spec:expr, $path_to_check_str:expr, $is_dir:expr, $expected_ignored:expr) => {
            let repo_root = rp($repo_root_str);
            let mut set = GitignoreSet::new(repo_root.clone());
            let rules_vec: Vec<(&str, &str)> = $rules_spec;

            for (line, gitignore_path_str_val) in rules_vec {
                let gitignore_abs_path = rp(gitignore_path_str_val);
                let gitignore_dir = gitignore_abs_path.parent().unwrap_or(&repo_root);
                if let Some(rule) = parse_line_to_rule(line, 1, &gitignore_abs_path, gitignore_dir) {
                    set.add_rule(rule);
                } else if !(line.trim().starts_with('#') || line.trim().is_empty()) {
                    panic!("Rule '{line}' in '{gitignore_path_str_val}' failed to parse but was expected to be valid.");
                }
            }

            let path_to_check = rp($path_to_check_str);
            let actual_ignored = !matches!(set.is_ignored(&path_to_check, $is_dir), GitIgnoreResult::NotIgnored);

            assert_eq!(
                actual_ignored,
                $expected_ignored,
                "Mismatch for path '{}' (is_dir: {}). Expected ignored: {}, Actual ignored: {}. Rules: {:?}",
                $path_to_check_str,
                $is_dir,
                $expected_ignored,
                actual_ignored,
                set.rules.iter().map(|r| &r.raw_pattern).collect::<Vec<_>>()
            );
        };
    }

    macro_rules! test_parse_rule {
        ($line:literal, $gitignore_path_str:literal, $expected:expr) => {{
            let line: &str = $line;
            let gitignore_path_str: &str = $gitignore_path_str;
            let expected: Option<GitignoreRule> = $expected;
            let gitignore_path = rp(gitignore_path_str);
            let base_path = gitignore_path.parent().unwrap().to_path_buf();
            let rule_opt =
                parse_line_to_rule(line, 1, &gitignore_path, &base_path);

            match (rule_opt, expected) {
                (Some(r), Some(e)) => {
                    assert_eq!(
                        r.raw_pattern, e.raw_pattern,
                        "raw_pattern mismatch for '{line}'"
                    );
                    assert_eq!(
                        r.tokens, e.tokens,
                        "tokens mismatch for '{line}'"
                    );
                    assert_eq!(
                        r.is_negation, e.is_negation,
                        "is_negation mismatch for '{line}'"
                    );
                    assert_eq!(
                        r.target_type, e.target_type,
                        "target_type mismatch for '{line}'"
                    );
                    assert_eq!(
                        r.base_path, e.base_path,
                        "base_path mismatch for '{line}'"
                    );
                    assert_eq!(
                        r.prepend_double_asterisk, e.prepend_double_asterisk,
                        "prepend_double_asterisk mismatch for '{line}'"
                    );
                }
                (None, None) => { /* Correctly no rule */ }
                (Some(r), None) => {
                    panic!("Expected no rule for '{line}', but got one: {r:?}")
                }
                (None, Some(_)) => {
                    panic!("Expected a rule for '{line}', but got none.")
                }
            }
        }};
    }

    #[test]
    #[expect(clippy::too_many_lines, reason = "GitignoreSet is pretty big")]
    fn test_parse_line_to_rule_basics() {
        let base_repo_path = rp("/repo");
        test_parse_rule!("# comment", "/repo/.gitignore", None);
        test_parse_rule!(
            "foo",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "foo".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::Segment(vec![
                    Atom::Literal("foo".to_string()),
                ])],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: true,
            })
        );
        test_parse_rule!(
            "!foo",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "!foo".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::Segment(vec![
                    Atom::Literal("foo".to_string()),
                ])],
                is_negation: true,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: true,
            })
        );
        test_parse_rule!(
            "foo/",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "foo/".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::Segment(vec![
                    Atom::Literal("foo".to_string()),
                ])],
                is_negation: false,
                target_type: PatternTargetType::DirectoryOnly,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: false,
            })
        );
        test_parse_rule!(
            "/foo",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "/foo".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::Segment(vec![
                    Atom::Literal("foo".to_string()),
                ])],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: false,
            })
        );
        test_parse_rule!(
            "foo/bar",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "foo/bar".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![
                    PathSegmentPatternToken::Segment(vec![Atom::Literal(
                        "foo".to_string(),
                    )]),
                    PathSegmentPatternToken::Segment(vec![Atom::Literal(
                        "bar".to_string(),
                    )]),
                ],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: false,
            })
        );
        test_parse_rule!(
            "/",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "/".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![],
                is_negation: false,
                target_type: PatternTargetType::DirectoryOnly,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: false,
            })
        );
    }

    #[test]
    fn test_parse_trailing_spaces() {
        let base_repo_path = rp("/repo");
        test_parse_rule!(
            "foo   ",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "foo   ".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::Segment(vec![
                    Atom::Literal("foo".to_string()),
                ])],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: true,
            })
        );
        test_parse_rule!(
            "foo\\ \\  ", // Input: "foo<BACKSLASH><SPACE><BACKSLASH><SPACE><SPACE>"
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "foo\\ \\  ".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::Segment(vec![
                    Atom::Literal("foo  ".to_string()), // Expected: "foo<SPACE><SPACE>"
                ])],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: true,
            })
        );
    }

    #[test]
    fn test_parse_double_asterisk() {
        let base_repo_path = rp("/repo");
        test_parse_rule!(
            "**/foo",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "**/foo".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![
                    PathSegmentPatternToken::DoubleAsterisk,
                    PathSegmentPatternToken::Segment(vec![Atom::Literal(
                        "foo".to_string(),
                    )]),
                ],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: false,
            })
        );
        test_parse_rule!(
            "**",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "**".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::DoubleAsterisk],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: false,
            })
        );
    }

    #[test]
    fn test_parse_segment_to_atoms_escaped() {
        assert_eq!(
            parse_segment_to_atoms("\\*foo"),
            vec![Atom::Literal("*foo".to_string())]
        );
        assert_eq!(
            parse_segment_to_atoms("foo\\?"),
            vec![Atom::Literal("foo?".to_string())]
        );
        assert_eq!(
            parse_segment_to_atoms("foo\\"),
            vec![Atom::Literal("foo\\".to_string())]
        );
        assert_eq!(
            parse_segment_to_atoms("foo\\\\bar"),
            vec![Atom::Literal("foo\\bar".to_string())]
        );
    }

    #[test]
    fn test_parse_segment_to_atoms_char_class() {
        assert_eq!(
            parse_segment_to_atoms("[\\]]"),
            vec![Atom::CharClass {
                members: vec![']'],
                inverted: false
            }]
        );
        assert_eq!(
            parse_segment_to_atoms("[a-cxyz]"),
            vec![Atom::CharClass {
                members: vec!['a', 'b', 'c', 'x', 'y', 'z'],
                inverted: false
            }]
        );
        assert_eq!(
            parse_segment_to_atoms("[!a-cxyz]"),
            vec![Atom::CharClass {
                members: vec!['a', 'b', 'c', 'x', 'y', 'z'],
                inverted: true
            }]
        );
        assert_eq!(
            parse_segment_to_atoms("[--z]"),
            vec![Atom::CharClass {
                members: vec!['-', 'z'],
                inverted: false
            }]
        );
        assert_eq!(
            parse_segment_to_atoms("[a-c-f-h]"),
            vec![Atom::CharClass {
                members: vec!['-', 'a', 'b', 'c', 'f', 'g', 'h'],
                inverted: false
            }]
        );
    }

    #[test]
    fn test_prepend_double_asterisk_logic() {
        let base_repo_path = rp("/repo");
        test_parse_rule!(
            "*",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "*".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::Segment(vec![
                    Atom::Asterisk,
                ])],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: true,
            })
        );
        test_parse_rule!(
            "a*b",
            "/repo/.gitignore",
            Some(GitignoreRule {
                raw_pattern: "a*b".to_string(),
                line_number: 0,
                file_path: base_repo_path.clone(),
                tokens: vec![PathSegmentPatternToken::Segment(vec![
                    Atom::Literal("a".to_string()),
                    Atom::Asterisk,
                    Atom::Literal("b".to_string()),
                ])],
                is_negation: false,
                target_type: PatternTargetType::FileOrDirectory,
                base_path: base_repo_path.clone(),
                prepend_double_asterisk: true,
            })
        );
    }

    #[test]
    fn test_is_ignored_simple_cases() {
        check_ignore_assert!(
            "/repo",
            vec![("!foo.c", "/repo/.gitignore"), ("*.c", "/repo/.gitignore")],
            "/repo/foo.c",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("*.c", "/repo/.gitignore"), ("!foo.c", "/repo/.gitignore")],
            "/repo/foo.c",
            false,
            false
        );
    }

    #[test]
    fn test_is_ignored_relative_gitignore() {
        check_ignore_assert!(
            "/repo",
            vec![("/bar.c", "/repo/foo/.gitignore")],
            "/repo/foo/bar.c",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("/bar.c", "/repo/foo/.gitignore")],
            "/repo/foo/subdir/bar.c",
            false,
            false
        );
        check_ignore_assert!(
            "/repo",
            vec![("bar.c", "/repo/foo/.gitignore")],
            "/repo/foo/bar.c",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("bar.c", "/repo/foo/.gitignore")],
            "/repo/foo/subdir/bar.c",
            false,
            true
        );
    }

    #[test]
    fn test_parse_segment_to_atoms_simple() {
        assert_eq!(
            parse_segment_to_atoms("foo"),
            vec![Atom::Literal("foo".to_string())]
        );
        assert_eq!(
            parse_segment_to_atoms("*.c"),
            vec![Atom::Asterisk, Atom::Literal(".c".to_string())]
        );
        assert_eq!(
            parse_segment_to_atoms("foo***bar"),
            vec![
                Atom::Literal("foo".to_string()),
                Atom::Asterisk,
                Atom::Literal("bar".to_string())
            ]
        );
    }

    #[test]
    fn test_match_atoms_to_segment_literal() {
        let atoms = vec![Atom::Literal("foo".to_string())];
        assert!(GitignoreSet::match_atoms_to_segment(&atoms, "foo", 0, 0));
        assert!(!GitignoreSet::match_atoms_to_segment(&atoms, "bar", 0, 0));
    }

    #[test]
    fn test_match_atoms_to_segment_asterisk() {
        let atoms1 = vec![Atom::Asterisk];
        assert!(GitignoreSet::match_atoms_to_segment(&atoms1, "", 0, 0));
        assert!(GitignoreSet::match_atoms_to_segment(&atoms1, "foo", 0, 0));
        let atoms2 = vec![Atom::Literal("foo".to_string()), Atom::Asterisk];
        assert!(GitignoreSet::match_atoms_to_segment(
            &atoms2, "foobar", 0, 0
        ));
    }

    #[test]
    fn test_match_atoms_to_segment_question_mark() {
        let atoms1 = vec![Atom::QuestionMark];
        assert!(GitignoreSet::match_atoms_to_segment(&atoms1, "a", 0, 0));
        assert!(!GitignoreSet::match_atoms_to_segment(&atoms1, "ab", 0, 0));
    }

    #[test]
    fn test_match_atoms_to_segment_char_class_extended() {
        let atoms1 = vec![Atom::CharClass {
            members: vec!['a', 'b', 'c'],
            inverted: false,
        }];
        assert!(GitignoreSet::match_atoms_to_segment(&atoms1, "a", 0, 0));
        assert!(!GitignoreSet::match_atoms_to_segment(&atoms1, "d", 0, 0));
        let atoms2 = vec![Atom::CharClass {
            members: vec!['a', 'b', 'c'],
            inverted: true,
        }];
        assert!(GitignoreSet::match_atoms_to_segment(&atoms2, "d", 0, 0));
        assert!(!GitignoreSet::match_atoms_to_segment(&atoms2, "a", 0, 0));
    }

    #[test]
    fn test_is_ignored_simple_cases_extended() {
        check_ignore_assert!(
            "/repo",
            vec![("foo.c", "/repo/.gitignore")],
            "/repo/foo.c",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("*.c", "/repo/.gitignore")],
            "/repo/subdir/bar.c",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("foo/", "/repo/.gitignore")],
            "/repo/foo",
            true,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("foo/", "/repo/.gitignore")],
            "/repo/foo",
            false,
            false
        );
        check_ignore_assert!(
            "/repo",
            vec![("/foo", "/repo/.gitignore")],
            "/repo/foo",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("/foo", "/repo/.gitignore")],
            "/repo/subdir/foo",
            false,
            false
        );
    }

    #[test]
    fn test_is_ignored_double_asterisk() {
        check_ignore_assert!(
            "/repo",
            vec![("**/foo", "/repo/.gitignore")],
            "/repo/a/foo",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("a/**/b", "/repo/.gitignore")],
            "/repo/a/x/y/b",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("foo/**", "/repo/.gitignore")],
            "/repo/foo/bar.txt",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("foo/**", "/repo/.gitignore")],
            "/repo/foo",
            true,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("a/b/**", "/repo/.gitignore")],
            "/repo/a/b/c/d.txt",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("a/b/**", "/repo/.gitignore")],
            "/repo/a/b/c",
            true,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("a/b/**", "/repo/.gitignore")],
            "/repo/a/b/file.txt",
            false,
            true
        );
    }

    #[test]
    fn test_is_ignored_original_pattern_core_had_no_slashes_behavior() {
        check_ignore_assert!(
            "/repo",
            vec![("foo.c", "/repo/.gitignore")],
            "/repo/a/foo.c",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("/foo.c", "/repo/.gitignore")],
            "/repo/a/foo.c",
            false,
            false
        );
    }

    #[test]
    fn test_is_ignored_root_matching() {
        check_ignore_assert!(
            "/repo",
            vec![("/", "/repo/.gitignore")],
            "/repo",
            true,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("/", "/repo/.gitignore")],
            "/repo",
            false,
            false
        );
        check_ignore_assert!(
            "/repo",
            vec![("/", "/repo/a/.gitignore")],
            "/repo/a",
            true,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("foo", "/repo/sub/.gitignore")],
            "/repo/sub/foo",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("foo", "/repo/sub/.gitignore")],
            "/repo/sub/bar/foo",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("/foo", "/repo/sub/.gitignore")],
            "/repo/sub/foo",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("/foo", "/repo/sub/.gitignore")],
            "/repo/foo",
            false,
            false
        );
    }

    #[test]
    fn test_is_ignored_complex_ordering_and_negation() {
        let rules1 =
            vec![("!foo.c", "/repo/.gitignore"), ("*.c", "/repo/.gitignore")];
        check_ignore_assert!("/repo", rules1, "/repo/foo.c", false, true);

        let rules2 =
            vec![("*.c", "/repo/.gitignore"), ("!foo.c", "/repo/.gitignore")];
        check_ignore_assert!("/repo", rules2, "/repo/foo.c", false, false);

        let rules_complex = vec![
            ("*.log", "/repo/.gitignore"),
            ("!important.log", "/repo/.gitignore"),
            ("debug/*.log", "/repo/.gitignore"),
            ("!debug/very_important.log", "/repo/.gitignore"),
        ];
        check_ignore_assert!(
            "/repo",
            rules_complex.clone(),
            "/repo/debug/important.log",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            rules_complex,
            "/repo/debug/very_important.log",
            false,
            false
        );
    }

    #[test]
    fn test_empty_and_comment_only_gitignore() {
        check_ignore_assert!(
            "/repo",
            vec![],
            "/repo/some/file.txt",
            false,
            false
        );
        check_ignore_assert!(
            "/repo",
            vec![
                ("# This is a comment", "/repo/.gitignore"),
                ("   # Another comment", "/repo/.gitignore")
            ],
            "/repo/another/file.txt",
            false,
            false
        );
    }

    #[test]
    fn test_utf8_patterns_and_paths() {
        check_ignore_assert!(
            "/repo",
            vec![("café.txt", "/repo/.gitignore")],
            "/repo/café.txt",
            false,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("你好世界/", "/repo/.gitignore")],
            "/repo/你好世界",
            true,
            true
        );
        check_ignore_assert!(
            "/repo",
            vec![("images/*.jpeg", "/repo/.gitignore")],
            "/repo/images/photo_你好.jpeg",
            false,
            true
        );
    }

    #[test]
    fn test_large_number_of_rules() {
        let repo_root_path = rp("/repo");
        let mut set = GitignoreSet::new(repo_root_path.clone());
        let gitignore_dir_path = repo_root_path.clone();

        for i in 0..100 {
            let pattern = format!("file_{i}.txt");
            if let Some(rule) = parse_line_to_rule(
                &pattern,
                0,
                &gitignore_dir_path,
                &gitignore_dir_path,
            ) {
                set.add_rule(rule);
            }
            let pattern_neg = format!("!important_file_{i}.txt");
            if let Some(rule) = parse_line_to_rule(
                &pattern_neg,
                0,
                &gitignore_dir_path,
                &gitignore_dir_path,
            ) {
                set.add_rule(rule);
            }
        }
        assert_ne!(
            set.is_ignored(&rp("/repo/sub/file_50.txt"), false),
            GitIgnoreResult::NotIgnored
        );
        assert_eq!(
            set.is_ignored(&rp("/repo/sub/important_file_50.txt"), false),
            GitIgnoreResult::NotIgnored
        );

        if let Some(rule) = parse_line_to_rule(
            "*.txt",
            0,
            &gitignore_dir_path,
            &gitignore_dir_path,
        ) {
            set.add_rule(rule);
        }
        assert_ne!(
            set.is_ignored(&rp("/repo/sub/important_file_50.txt"), false),
            GitIgnoreResult::NotIgnored
        );

        if let Some(rule) = parse_line_to_rule(
            "!specific_important_file.txt",
            0,
            &gitignore_dir_path,
            &gitignore_dir_path,
        ) {
            set.add_rule(rule);
        }
        assert_eq!(
            set.is_ignored(&rp("/repo/sub/specific_important_file.txt"), false),
            GitIgnoreResult::NotIgnored
        );
    }
}
