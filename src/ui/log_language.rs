//! Custom tree-sitter language for the BepInEx log viewer.
//!
//! gpui-component's `InputState::code_editor(language)` looks the language up
//! in [`LanguageRegistry::singleton`]. Tree-sitter doesn't ship a log grammar,
//! so we register two cooperating entries built on top of `tree-sitter-md`:
//!
//! * `log` uses the **block** grammar (`LANGUAGE`). Its only job is to split
//!   the log into paragraphs / inline runs and inject `log_inline` into each.
//! * `log_inline` uses the **inline** grammar (`INLINE_LANGUAGE`). That's
//!   where our level-based highlights actually fire — `[Info: …]` parses to
//!   a `shortcut_link` whose `link_text` we match against `Error`, `Warning`,
//!   etc. via `#match?` predicates.

use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};

/// Highlights for the inline grammar — the per-log-level coloring lives here.
///
/// Tree-sitter query reference:
/// https://tree-sitter.github.io/tree-sitter/using-parsers/queries
const INLINE_HIGHLIGHTS: &str = r#"
((link_text) @keyword
  (#match? @keyword "^\\s*Error"))

((link_text) @constant
  (#match? @constant "^\\s*Warning"))

((link_text) @type
  (#match? @type "^\\s*Info"))

((link_text) @function
  (#match? @function "^\\s*Message"))

((link_text) @comment
  (#match? @comment "^\\s*Debug"))

; Color the surrounding `[` `]` brackets so the prefix reads as a unit.
[
  "["
  "]"
] @punctuation.bracket
"#;

/// Block-level injection: hand every `(inline)` node off to `log_inline`.
const BLOCK_INJECTIONS: &str = r#"
((inline) @injection.content (#set! injection.language "log_inline"))
"#;

/// Register the `"log"` language (and its `"log_inline"` injection partner)
/// with the global [`LanguageRegistry`] so they can be selected via
/// `InputState::code_editor("log")`. Safe to call more than once —
/// re-registering simply replaces the previous entries.
pub fn register() {
    let registry = LanguageRegistry::singleton();

    let inline_language: tree_sitter::Language = tree_sitter_md::INLINE_LANGUAGE.into();
    let inline_config = LanguageConfig::new(
        "log_inline",
        inline_language,
        vec![],
        INLINE_HIGHLIGHTS,
        "",
        "",
    );
    registry.register("log_inline", &inline_config);

    let block_language: tree_sitter::Language = tree_sitter_md::LANGUAGE.into();
    let block_config = LanguageConfig::new(
        "log",
        block_language,
        vec!["log_inline".into()],
        "",
        BLOCK_INJECTIONS,
        "",
    );
    registry.register("log", &block_config);
}
