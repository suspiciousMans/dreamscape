mod interpreter;
mod lexer;
mod parser;

pub use interpreter::{Host, Interpreter, NoHost, Value};

/// A parse-time or runtime error. `line` is only meaningful for parse
/// errors — runtime errors (undefined variable, wrong argument count, ...)
/// report `0` since script statements don't carry their source line once
/// past parsing; good enough for a "simple" language, worth revisiting if
/// runtime debugging ever needs it.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptError {
    pub message: String,
    pub line: u32,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ScriptError {}
