use crossterm::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightType {
    None,
    Keyword,
    Type,
    String,
    Comment,
    Number,
}

impl HighlightType {
    pub fn foreground_color(&self) -> Option<Color> {
        match self {
            HighlightType::Keyword => Some(Color::Magenta),
            HighlightType::Type => Some(Color::Cyan),
            HighlightType::String => Some(Color::Green),
            HighlightType::Comment => Some(Color::DarkGrey),
            HighlightType::Number => Some(Color::Yellow),
            HighlightType::None => None,
        }
    }
}

pub struct LanguageConfig {
    pub name: &'static str,
    pub keywords: &'static [&'static str],
    pub type_keywords: &'static [&'static str],
    pub string_delimiters: &'static [(&'static str, &'static str)],
    pub line_comment: Option<&'static str>,
    pub block_comment: Option<(&'static str, &'static str)>,
}

// ---- Language Definitions ----

static RUST: LanguageConfig = LanguageConfig {
    name: "Rust",
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn",
        "else", "enum", "extern", "fn", "for", "if", "impl", "in", "let",
        "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
        "self", "Self", "static", "struct", "super", "trait", "type",
        "unsafe", "use", "where", "while", "yield", "true", "false",
    ],
    type_keywords: &[
        "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128",
        "isize", "str", "u8", "u16", "u32", "u64", "u128", "usize",
        "String", "Vec", "Option", "Result", "Box", "Rc", "Arc",
    ],
    string_delimiters: &[("\"", "\""), ("'", "'")],
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
};

static PYTHON: LanguageConfig = LanguageConfig {
    name: "Python",
    keywords: &[
        "False", "None", "True", "and", "as", "assert", "async", "await",
        "break", "class", "continue", "def", "del", "elif", "else", "except",
        "finally", "for", "from", "global", "if", "import", "in", "is",
        "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try",
        "while", "with", "yield",
    ],
    type_keywords: &[
        "int", "float", "str", "bool", "list", "dict", "set", "tuple",
        "bytes", "bytearray", "complex", "frozenset", "range", "type",
        "object", "Exception", "BaseException",
    ],
    string_delimiters: &[("\"", "\""), ("'", "'"), ("\"\"\"", "\"\"\""), ("'''", "'''")],
    line_comment: Some("#"),
    block_comment: None,
};

static C: LanguageConfig = LanguageConfig {
    name: "C",
    keywords: &[
        "auto", "break", "case", "char", "const", "continue", "default", "do",
        "double", "else", "enum", "extern", "float", "for", "goto", "if",
        "int", "long", "register", "return", "short", "signed", "sizeof",
        "static", "struct", "switch", "typedef", "union", "unsigned", "void",
        "volatile", "while", "true", "false", "NULL",
    ],
    type_keywords: &[
        "size_t", "ptrdiff_t", "int8_t", "int16_t", "int32_t", "int64_t",
        "uint8_t", "uint16_t", "uint32_t", "uint64_t", "FILE",
    ],
    string_delimiters: &[("\"", "\""), ("'", "'")],
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
};

static CPP: LanguageConfig = LanguageConfig {
    name: "C++",
    keywords: &[
        "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand",
        "bitor", "bool", "break", "case", "catch", "char", "char8_t",
        "char16_t", "char32_t", "class", "compl", "concept", "const",
        "consteval", "constexpr", "const_cast", "continue", "co_await",
        "co_return", "co_yield", "decltype", "default", "delete", "do",
        "double", "dynamic_cast", "else", "enum", "explicit", "export",
        "extern", "false", "float", "for", "friend", "goto", "if", "inline",
        "int", "long", "mutable", "namespace", "new", "noexcept", "not",
        "not_eq", "nullptr", "operator", "or", "or_eq", "private",
        "protected", "public", "register", "reinterpret_cast", "requires",
        "return", "short", "signed", "sizeof", "static", "static_assert",
        "static_cast", "struct", "switch", "template", "this", "thread_local",
        "throw", "true", "try", "typedef", "typeid", "typename", "union",
        "unsigned", "using", "virtual", "void", "volatile", "wchar_t",
        "while", "xor", "xor_eq", "override", "final",
    ],
    type_keywords: &[
        "size_t", "string", "vector", "map", "set", "unordered_map",
        "unordered_set", "pair", "tuple", "array", "deque", "list",
        "queue", "stack", "priority_queue", "shared_ptr", "unique_ptr",
        "weak_ptr", "optional", "variant", "any",
    ],
    string_delimiters: &[("\"", "\""), ("'", "'")],
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
};

static JAVASCRIPT: LanguageConfig = LanguageConfig {
    name: "JavaScript",
    keywords: &[
        "async", "await", "break", "case", "catch", "class", "const",
        "continue", "debugger", "default", "delete", "do", "else", "export",
        "extends", "false", "finally", "for", "function", "if", "import",
        "in", "instanceof", "let", "new", "null", "of", "return", "static",
        "super", "switch", "this", "throw", "true", "try", "typeof",
        "undefined", "var", "void", "while", "with", "yield",
    ],
    type_keywords: &[
        "Array", "Boolean", "Date", "Error", "Function", "Map", "Number",
        "Object", "Promise", "Proxy", "RegExp", "Set", "String", "Symbol",
        "WeakMap", "WeakSet", "BigInt", "console",
    ],
    string_delimiters: &[("\"", "\""), ("'", "'"), ("`", "`")],
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
};

static TYPESCRIPT: LanguageConfig = LanguageConfig {
    name: "TypeScript",
    keywords: &[
        "abstract", "any", "as", "async", "await", "break", "case", "catch",
        "class", "const", "continue", "debugger", "default", "delete", "do",
        "else", "enum", "export", "extends", "false", "finally", "for",
        "from", "function", "if", "implements", "import", "in", "instanceof",
        "interface", "let", "module", "new", "null", "of", "package",
        "private", "protected", "public", "readonly", "require", "return",
        "static", "super", "switch", "this", "throw", "true", "try", "type",
        "typeof", "undefined", "unique", "unknown", "var", "void", "while",
        "with", "yield",
    ],
    type_keywords: &[
        "Array", "Boolean", "Date", "Error", "Function", "Map", "Never",
        "Number", "Object", "Promise", "Record", "RegExp", "Set", "String",
        "Symbol", "Tuple", "WeakMap", "WeakSet", "console",
    ],
    string_delimiters: &[("\"", "\""), ("'", "'"), ("`", "`")],
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
};

static JAVA: LanguageConfig = LanguageConfig {
    name: "Java",
    keywords: &[
        "abstract", "assert", "boolean", "break", "byte", "case", "catch",
        "char", "class", "const", "continue", "default", "do", "double",
        "else", "enum", "extends", "final", "finally", "float", "for",
        "goto", "if", "implements", "import", "instanceof", "int",
        "interface", "long", "native", "new", "package", "private",
        "protected", "public", "return", "short", "static", "strictfp",
        "super", "switch", "synchronized", "this", "throw", "throws",
        "transient", "try", "void", "volatile", "while", "true", "false",
        "null",
    ],
    type_keywords: &[
        "String", "Integer", "Long", "Double", "Float", "Boolean", "Byte",
        "Short", "Character", "Object", "Class", "Thread", "Runnable",
        "Exception", "ArrayList", "HashMap", "HashSet", "LinkedList",
        "Optional", "Stream", "List", "Map", "Set", "Queue", "Deque",
    ],
    string_delimiters: &[("\"", "\""), ("'", "'")],
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
};

static GO: LanguageConfig = LanguageConfig {
    name: "Go",
    keywords: &[
        "break", "case", "chan", "const", "continue", "default", "defer",
        "else", "fallthrough", "for", "func", "go", "goto", "if", "import",
        "interface", "map", "package", "range", "return", "select", "struct",
        "switch", "type", "var", "true", "false", "nil", "append", "cap",
        "close", "copy", "delete", "len", "make", "new", "panic", "print",
        "println", "recover",
    ],
    type_keywords: &[
        "bool", "byte", "complex64", "complex128", "error", "float32",
        "float64", "int", "int8", "int16", "int32", "int64", "rune",
        "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
    ],
    string_delimiters: &[("\"", "\""), ("`", "`"), ("'", "'")],
    line_comment: Some("//"),
    block_comment: None,
};

static SHELL: LanguageConfig = LanguageConfig {
    name: "Shell",
    keywords: &[
        "if", "then", "else", "elif", "fi", "case", "esac", "for", "while",
        "until", "do", "done", "in", "function", "select", "time", "coproc",
        "return", "exit", "break", "continue", "declare", "export", "local",
        "readonly", "typeset", "unset", "source", "alias", "bg", "fg",
        "jobs", "kill", "wait", "read", "echo", "printf", "cd", "pwd",
        "true", "false",
    ],
    type_keywords: &[],
    string_delimiters: &[("\"", "\""), ("'", "'")],
    line_comment: Some("#"),
    block_comment: None,
};

static JSON: LanguageConfig = LanguageConfig {
    name: "JSON",
    keywords: &["true", "false", "null"],
    type_keywords: &[],
    string_delimiters: &[("\"", "\"")],
    line_comment: None,
    block_comment: None,
};

static TOML: LanguageConfig = LanguageConfig {
    name: "TOML",
    keywords: &["true", "false"],
    type_keywords: &[],
    string_delimiters: &[("\"", "\""), ("'", "'")],
    line_comment: Some("#"),
    block_comment: None,
};

static HTML: LanguageConfig = LanguageConfig {
    name: "HTML",
    keywords: &[],
    type_keywords: &[],
    string_delimiters: &[("\"", "\""), ("'", "'")],
    line_comment: None,
    block_comment: Some(("<!--", "-->")),
};

static CSS: LanguageConfig = LanguageConfig {
    name: "CSS",
    keywords: &[
        "align-content", "align-items", "align-self", "animation", "background",
        "border", "bottom", "box-shadow", "box-sizing", "clear", "color",
        "content", "cursor", "display", "flex", "flex-direction", "flex-wrap",
        "float", "font", "font-family", "font-size", "font-weight", "grid",
        "grid-template-columns", "grid-template-rows", "height", "justify-content",
        "left", "letter-spacing", "line-height", "margin", "max-height",
        "max-width", "min-height", "min-width", "opacity", "order", "outline",
        "overflow", "padding", "pointer-events", "position", "resize", "right",
        "text-align", "text-decoration", "text-transform", "top", "transform",
        "transition", "user-select", "vertical-align", "visibility",
        "white-space", "width", "word-wrap", "z-index",
        "important", "inherit", "initial", "none", "auto", "block", "inline",
        "flex", "grid", "absolute", "relative", "fixed", "sticky", "hidden",
        "visible", "center", "left", "right", "top", "bottom", "solid",
        "dashed", "normal", "bold", "uppercase", "lowercase", "capitalize",
    ],
    type_keywords: &["@media", "@keyframes", "@import", "@font-face", "@charset", "@supports"],
    string_delimiters: &[("\"", "\""), ("'", "'")],
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
};

pub fn detect_language(filename: &str) -> Option<&'static LanguageConfig> {
    let ext = filename.rsplit('.').next()?;
    match ext {
        "rs" => Some(&RUST),
        "py" | "pyw" => Some(&PYTHON),
        "c" | "h" => Some(&C),
        "cpp" | "hpp" | "cc" | "cxx" => Some(&CPP),
        "js" | "mjs" | "jsx" => Some(&JAVASCRIPT),
        "ts" | "tsx" => Some(&TYPESCRIPT),
        "java" => Some(&JAVA),
        "go" => Some(&GO),
        "sh" | "bash" | "zsh" => Some(&SHELL),
        "json" => Some(&JSON),
        "toml" => Some(&TOML),
        "html" | "htm" => Some(&HTML),
        "css" | "scss" | "sass" | "less" => Some(&CSS),
        _ => None,
    }
}

/// Returns per-char highlight types and whether we end in a block comment
pub fn highlight_row(
    chars: &[char],
    lang: &LanguageConfig,
    in_block_comment: bool,
) -> (Vec<HighlightType>, bool) {
    let len = chars.len();
    let mut highlights = vec![HighlightType::None; len];
    let mut in_block = in_block_comment;
    let mut in_string: Option<&str> = None;
    let mut i = 0;

    while i < len {
        // Inside block comment
        if in_block {
            if let Some((_, end)) = lang.block_comment {
                let end_chars: Vec<char> = end.chars().collect();
                if i + end_chars.len() <= len && chars[i..i + end_chars.len()] == end_chars[..] {
                    for j in i..i + end_chars.len() {
                        highlights[j] = HighlightType::Comment;
                    }
                    i += end_chars.len();
                    in_block = false;
                    continue;
                }
            }
            highlights[i] = HighlightType::Comment;
            i += 1;
            continue;
        }

        // Inside string
        if let Some(end_delim) = in_string {
            let end_chars: Vec<char> = end_delim.chars().collect();
            // Check for escape
            if chars[i] == '\\' && i + 1 < len {
                highlights[i] = HighlightType::String;
                highlights[i + 1] = HighlightType::String;
                i += 2;
                continue;
            }
            highlights[i] = HighlightType::String;
            if i + end_chars.len() <= len && chars[i..i + end_chars.len()] == end_chars[..] {
                for j in 1..end_chars.len() {
                    highlights[i + j] = HighlightType::String;
                }
                i += end_chars.len();
                in_string = None;
            } else {
                i += 1;
            }
            continue;
        }

        // Check for string start (try longer delimiters first)
        let mut found_string = false;
        for (start, end) in lang.string_delimiters {
            let start_chars: Vec<char> = start.chars().collect();
            if i + start_chars.len() <= len && chars[i..i + start_chars.len()] == start_chars[..] {
                for j in 0..start_chars.len() {
                    highlights[i + j] = HighlightType::String;
                }
                i += start_chars.len();
                in_string = Some(*end);
                found_string = true;
                break;
            }
        }
        if found_string {
            continue;
        }

        // Check for line comment
        if let Some(comment_start) = lang.line_comment {
            let cs_chars: Vec<char> = comment_start.chars().collect();
            if i + cs_chars.len() <= len && chars[i..i + cs_chars.len()] == cs_chars[..] {
                for j in i..len {
                    highlights[j] = HighlightType::Comment;
                }
                break;
            }
        }

        // Check for block comment start
        if let Some((start, _)) = lang.block_comment {
            let sc_chars: Vec<char> = start.chars().collect();
            if i + sc_chars.len() <= len && chars[i..i + sc_chars.len()] == sc_chars[..] {
                for j in 0..sc_chars.len() {
                    highlights[i + j] = HighlightType::Comment;
                }
                i += sc_chars.len();
                in_block = true;
                continue;
            }
        }

        // Check for numbers
        if chars[i].is_ascii_digit() {
            let prev_ok = i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_' || chars[i - 1] == '.');
            if prev_ok {
                highlights[i] = HighlightType::Number;
                i += 1;
                while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'x'
                    || chars[i] == 'a' || chars[i] == 'b' || chars[i] == 'c'
                    || chars[i] == 'd' || chars[i] == 'e' || chars[i] == 'f'
                    || chars[i] == '_' || chars[i] == 'i' || chars[i] == 'u'
                    || chars[i] == 'f' || chars[i] == 'l')
                {
                    highlights[i] = HighlightType::Number;
                    i += 1;
                }
                continue;
            }
        }

        // Check for keywords/types (word boundary)
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let word_start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[word_start..i].iter().collect();
            if lang.keywords.contains(&word.as_str()) {
                for j in word_start..i {
                    highlights[j] = HighlightType::Keyword;
                }
            } else if lang.type_keywords.contains(&word.as_str()) {
                for j in word_start..i {
                    highlights[j] = HighlightType::Type;
                }
            }
            continue;
        }

        i += 1;
    }

    (highlights, in_block)
}
