use crate::{command::InputMode, repl_helper::QldbHelper};
use crate::{command::SetCommand, settings::Environment};
use anyhow::Result;
use dirs;
use rustyline::{
    config::Builder, error::ReadlineError, Cmd, EditMode, KeyCode, KeyEvent, Modifiers,
};
use rustyline::{Config, Editor};
use std::cell::RefCell;
use std::{io, path::PathBuf};
use tracing::{debug, warn};

pub(crate) trait Ui {
    fn set_prompt(&self, prompt: String);

    fn user_input(&self) -> Result<String>;

    fn clear_pending(&self);

    fn println(&self, str: &str);

    fn newline(&self);

    fn print(&self, str: &str);

    fn warn(&self, str: &str);

    fn debug(&self, str: &str);

    fn handle_env_set(&self, set: &SetCommand) -> Result<()>;
}

#[cfg(test)]
pub mod testing {
    use std::{cell::RefMut, sync::Arc};

    use super::*;

    #[derive(Default)]
    pub struct TestUiInner {
        pub prompt: String,
        pub pending: Vec<String>,
        pub output: Vec<String>,
        pub warn: Vec<String>,
        pub debug: Vec<String>,
    }

    #[derive(Default, Clone)]
    pub struct TestUi {
        pub inner: Arc<RefCell<TestUiInner>>,
    }

    impl TestUi {
        pub fn inner(&self) -> RefMut<'_, TestUiInner> {
            self.inner.borrow_mut()
        }
    }

    impl Ui for TestUi {
        fn set_prompt(&self, prompt: String) {
            self.inner.borrow_mut().prompt = prompt;
        }

        fn user_input(&self) -> Result<String> {
            let mut inner = self.inner.borrow_mut();
            if inner.pending.is_empty() {
                panic!("mock is not ready for user input");
            }
            let remaining = inner.pending.split_off(1);
            let first = inner.pending.pop().unwrap();
            inner.pending = remaining;
            return Ok(first);
        }

        fn clear_pending(&self) {
            self.inner.borrow_mut().pending.clear();
        }

        fn println(&self, str: &str) {
            self.inner.borrow_mut().output.push(str.to_string());
        }

        fn newline(&self) {
            self.inner.borrow_mut().output.push("\n".to_string());
        }

        fn print(&self, str: &str) {
            self.inner.borrow_mut().output.push(str.to_string());
        }

        fn warn(&self, str: &str) {
            self.inner.borrow_mut().warn.push(str.to_string());
        }

        fn debug(&self, str: &str) {
            self.inner.borrow_mut().debug.push(str.to_string());
        }

        fn handle_env_set(&self, _set: &SetCommand) -> Result<()> {
            unimplemented!()
        }
    }
}

struct UiInner {
    env: Environment,
    editor: Editor<QldbHelper>,
    prompt: String,
    pending_actions: Vec<String>,
}

/// Encapsulates handling of user input. In particular, we use readline to
/// handle keyboard input (capturing lines, but also history and Emacs/Vi
/// bindings) and support 'sending; multiple; inputs'. We also capture history
/// for uparrow or Ctrl-R replay.
///
/// This type users interior mutability because the [`QldbDriver::transact`]
/// method takes a `Fn` arg (not `FnMut`) so that retries don't have
/// side-effects. However, we disable retries and thus don't care about the
/// side-effects (e.g. of saving history).
pub(crate) struct ConsoleUi {
    inner: RefCell<UiInner>,
}

impl ConsoleUi {
    pub(crate) fn new(env: Environment) -> ConsoleUi {
        let mut editor = create_editor(create_config(&env), env.clone());

        if let Some(p) = history_path() {
            editor.load_history(&p).keep_going();
        }

        ConsoleUi {
            inner: RefCell::new(UiInner {
                env,
                editor,
                prompt: "> ".to_owned(),
                pending_actions: vec![],
            }),
        }
    }

    // This is a big hack. Some open questions:
    //
    // 1. How to support single statement transactions
    // 2. Really don't need all the readline stuff here
    // 3. Also don't want to load/persist history
    // 4. exit is awful
    pub(crate) fn new_for_script(script: &str, env: Environment) -> io::Result<ConsoleUi> {
        let editor = create_editor(create_config(&env), env.clone());

        // We start the pending actions by reading the input, splitting it up
        // into new lines..
        let mut pending_actions: Vec<_> = script
            .lines()
            .map(|line| line.split(";").map(|it| it.trim().to_owned()))
            .flatten()
            .collect();
        // ..and then adding an exit comment
        pending_actions.push("exit".to_string()); // totally not a hack.
        pending_actions.reverse(); // also not a hack

        Ok(ConsoleUi {
            inner: RefCell::new(UiInner {
                env,
                editor,
                prompt: "".to_owned(),
                pending_actions,
            }),
        })
    }
}

fn create_config(_env: &Environment) -> Builder {
    Config::builder()
}

fn create_editor(builder: Builder, env: Environment) -> Editor<QldbHelper> {
    let mut editor = Editor::with_config(builder.build());
    editor.set_helper(Some(QldbHelper::new(env)));
    editor.bind_sequence(force_newline_event_seq(), Cmd::Newline);
    editor
}

#[cfg(not(windows))]
fn force_newline_event_seq() -> KeyEvent {
    KeyEvent(KeyCode::Enter, Modifiers::ALT)
}

// On Windows, `SHIFT+ENTER` is the key sequence for forcing a newline. This is
// because `ALT+ENTER` typically maximizes the window.
#[cfg(windows)]
fn force_newline_event_seq() -> KeyEvent {
    KeyEvent(KeyCode::Enter, Modifiers::SHIFT)
}

impl Ui for ConsoleUi {
    fn set_prompt(&self, prompt: String) {
        self.inner.borrow_mut().prompt = prompt;
    }

    /// Prompts the user for input or returns the next pending action.
    ///
    /// Users can enter multiple commands like 'foo; bar'. These commands will
    /// be processed as if first 'foo' was entered and then 'bar', except that
    /// errors MUST halt the chain (see [`clear_pending`]).
    ///
    /// Note that the history will contain the actual input ('foo; bar' not
    /// 'foo' & 'bar'). Similarly, we trim the strings such that 'foo;bar' and
    /// 'foo; bar' are treated identically (but the history will have the raw
    /// input).
    fn user_input(&self) -> Result<String> {
        let mut inner = self.inner.borrow_mut();

        if !inner.pending_actions.is_empty() {
            return Ok(inner.pending_actions.pop().unwrap());
        }

        let prompt = inner.prompt.clone();
        match inner.editor.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if !line.is_empty() {
                    inner.editor.add_history_entry(line);
                }
                inner.pending_actions = line.split(";").map(|it| it.trim().to_owned()).collect();
                inner.pending_actions.reverse();
                drop(inner);
                self.user_input()
            }
            Err(e) => Err(e)?,
        }
    }

    /// Clear the queue of pending actions. This method should be called on error.
    fn clear_pending(&self) {
        self.inner.borrow_mut().pending_actions.clear();
    }

    fn println(&self, str: &str) {
        println!("{}", str);
    }

    fn newline(&self) {
        println!();
    }

    fn print(&self, str: &str) {
        print!("{}", str);
    }

    fn warn(&self, str: &str) {
        warn!("{}", str);
    }

    fn debug(&self, str: &str) {
        debug!("{}", str);
    }

    fn handle_env_set(&self, set: &SetCommand) -> Result<()> {
        let mut inner = self.inner.borrow_mut();

        match set {
            SetCommand::InputMode(mode) => {
                let builder = create_config(&inner.env);
                let builder = builder.edit_mode(match mode {
                    InputMode::Emacs => EditMode::Emacs,
                    InputMode::Vi => EditMode::Vi,
                });

                let editor = create_editor(builder, inner.env.clone());
                inner.editor = editor;
            }
        }

        Ok(())
    }
}

impl Drop for ConsoleUi {
    fn drop(&mut self) {
        if let Some(p) = history_path() {
            self.inner.borrow_mut().editor.save_history(&p).keep_going();
        }
    }
}

fn history_path() -> Option<PathBuf> {
    match dirs::home_dir() {
        Some(dir) => Some(dir.join(".qldbshell_history")),
        None => None,
    }
}

trait KeepGoing {
    fn keep_going(self) -> ();
}

impl KeepGoing for Result<(), ReadlineError> {
    fn keep_going(self) -> () {
        match self {
            Ok(_) => (),
            Err(e) => warn!("{}", e),
        }
    }
}
