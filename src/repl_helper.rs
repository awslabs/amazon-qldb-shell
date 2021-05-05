use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Context;
use rustyline::Result as RustylineResult;
use rustyline::{
    completion::{Completer, FilenameCompleter, Pair},
    validate::{ValidationContext, ValidationResult},
};
use rustyline_derive::Helper;
use std::{
    borrow::Cow::{self, Borrowed, Owned},
    fmt::Display,
};

use crate::settings::Environment;

#[derive(Helper)]
pub(crate) struct QldbHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: InputValidator,
    hinter: (),
}

impl QldbHelper {
    pub fn new(environment: Environment) -> QldbHelper {
        QldbHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: InputValidator::new(environment),
            hinter: (),
        }
    }
}

impl Completer for QldbHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for QldbHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for QldbHelper {
    /// Use the default for prompts like history search, else use a bold + color code. We use blue for 'not in a tx' and green for 'in a tx'. Hopefully this is color blind friendly.
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            // FIXME: Use ansi crate
            // FIXME: Find another way of determining state (than substring matching)
            if prompt.contains("*") {
                Owned(format!("\x1b[1;32m{}\x1b[0m", prompt))
            } else {
                Owned(format!("\x1b[1;34m{}\x1b[0m", prompt))
            }
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl Validator for QldbHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> RustylineResult<ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

/// Mostly MatchingBracketHighlighter but with support for PartiQL bags. This
/// allows, primarily, for multi-line input of bags.
struct InputValidator {
    environment: Environment,
}

impl InputValidator {
    fn new(environment: Environment) -> InputValidator {
        InputValidator { environment }
    }
}

impl Validator for InputValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> RustylineResult<ValidationResult> {
        if self.environment.terminator_required().value {
            if !ctx.input().ends_with(";") {
                return Ok(ValidationResult::Incomplete);
            }
        }

        Ok(validate_structure(ctx.input()))
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}

#[derive(Debug, Eq, PartialEq)]
enum StructureCheck {
    Single(char),
    Repeat(char),
}

impl StructureCheck {
    fn starts(c: char, next: Option<&char>) -> Option<StructureCheck> {
        use StructureCheck::*;

        match c {
            '(' => Some(Single(')')),
            '[' => Some(Single(']')),
            '{' => Some(Single('}')),
            '<' => {
                if let Some('<') = next {
                    Some(Repeat('>'))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn completes(c: char, next: Option<&char>) -> Option<StructureCheck> {
        use StructureCheck::*;

        match c {
            ')' | ']' | '}' => Some(Single(c)),
            '>' => {
                if let Some('>') = next {
                    return Some(Repeat('>'));
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Display for StructureCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructureCheck::Single(c) => write!(f, "{}", c),
            StructureCheck::Repeat(c) => write!(f, "{}{}", c, c),
        }
    }
}

fn validate_structure(input: &str) -> ValidationResult {
    let mut stack = vec![];
    let mut iter = input.chars().peekable();
    while let Some(c) = iter.next() {
        if let Some(starts) = StructureCheck::starts(c, iter.peek()) {
            stack.push(starts);
        }

        if let Some(completes) = StructureCheck::completes(c, iter.peek()) {
            if let Some(top) = stack.pop() {
                if completes != top {
                    return ValidationResult::Invalid(Some(format!(
                        "Invalid input, expecting: {}",
                        top
                    )));
                }
            } else {
                return ValidationResult::Invalid(Some(format!(
                    "Invalid input: {} is unpaired",
                    completes
                )));
            }
        }
    }
    if stack.is_empty() {
        ValidationResult::Valid(None)
    } else {
        ValidationResult::Incomplete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ValidationResult doesn't implement Debug
    fn string(it: ValidationResult) -> String {
        match it {
            ValidationResult::Incomplete => format!("incomplete"),
            ValidationResult::Invalid(Some(m)) => format!("invalid: {}", m),
            ValidationResult::Valid(None) => format!("valid"),
            ValidationResult::Valid(Some(m)) => format!("valid: {}", m),
            _ => unreachable!(),
        }
    }

    macro_rules! assert_validates {
        ($expected:expr, $actual:expr) => {
            assert_eq!(
                string($expected),
                string(validate_structure($actual)),
                "{}",
                $actual
            );
        };
    }

    #[test]
    fn validate_structures() {
        // Simple, complete cases
        assert_validates!(ValidationResult::Valid(None), "");
        assert_validates!(ValidationResult::Valid(None), "hello world");
        assert_validates!(ValidationResult::Valid(None), "hello () [] {} << >> world");

        // Struture started but not completed.
        assert_validates!(ValidationResult::Incomplete, "hello (");
        assert_validates!(ValidationResult::Incomplete, "hello [");
        assert_validates!(ValidationResult::Incomplete, "hello {");
        assert_validates!(ValidationResult::Incomplete, "hello <<");
        // bag is <<
        assert_validates!(ValidationResult::Valid(None), "hello <");
    }
}
