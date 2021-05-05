pub use command_line::{ExecuteStatementOpt, FormatMode, Opt};
pub use config::Config;
pub use environment::Environment;

mod command_line;
pub mod config;
mod environment;

#[derive(Clone, Debug)]
pub enum Setter {
    Config,
    CommandLine,
    Environment,
}

#[derive(Clone, Debug)]
pub struct Setting<T: Clone> {
    name: String,
    modified: bool,
    setter: Setter,
    pub value: T,
}

impl<T> Setting<T>
where
    T: Clone,
{
    pub(crate) fn apply_value(&mut self, other: &T, setter: Setter) {
        self.modified = true;
        self.setter = setter;
        self.value = other.clone();
    }

    fn apply_value_opt(&mut self, other: &Option<T>, setter: Setter) {
        if let Some(value) = other {
            self.modified = true;
            self.setter = setter;
            self.value = value.clone();
        }
    }
}

impl<T> Setting<Option<T>>
where
    T: Clone,
{
    fn apply_opt(&mut self, other: &Option<T>, setter: Setter) {
        match (&self.value, other) {
            (None, None) => {}
            (Some(_), None) => {
                self.modified = true;
                self.setter = setter;
                self.value = None;
            }
            (_, Some(_)) => self.apply_value(other, setter),
        }
    }
}
