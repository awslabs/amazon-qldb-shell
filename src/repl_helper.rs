use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::Hinter;
use rustyline::validate::{self, MatchingBracketValidator, Validator};
use rustyline::Context;
use rustyline_derive::Helper;
use std::borrow::Cow::{self, Borrowed, Owned};

#[derive(Helper)]
pub(crate) struct ReplHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: (),
}

impl Default for ReplHelper {
    fn default() -> ReplHelper {
        ReplHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: (),
        }
    }
}

impl Completer for ReplHelper {
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

impl Hinter for ReplHelper {
    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for ReplHelper {
    /// Use the default for prompts like history search, else use a bold + color code. We use blue for 'not in a tx' and green for 'in a tx'. Hopefully this is color blind friendly.
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            // FIXME: Use ansi crate
            // FIXME: Find another way of determining state (than substring matching)
            if prompt.contains("tx") {
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

impl Validator for ReplHelper {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}
