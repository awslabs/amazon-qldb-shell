use crate::settings::{Setter, Setting};
use anyhow::Result;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "settings/command_line_options.pest"]
pub struct CommandLineOptionParser;

impl CommandLineOptionParser {
    pub fn parse_on_off(s: &str) -> Result<Setting<bool>> {
        let assignment = CommandLineOptionParser::parse(Rule::on_off_assignment, s)?
            .next()
            .unwrap();
        let mut rule = assignment.into_inner();
        let name = rule.next().unwrap().as_str();
        let value = match &rule.next().unwrap().as_str().to_lowercase()[..] {
            "on" => true,
            "off" => false,
            _ => unreachable!("by the grammar"),
        };

        Ok(Setting {
            name: name.to_string(),
            modified: true,
            setter: Setter::CommandLine,
            value,
        })
    }
}

#[cfg(test)]
mod settings_command_line_tests {
    use super::*;

    #[test]
    fn test_parse_on_off() -> Result<()> {
        assert_eq!(true, CommandLineOptionParser::parse_on_off("foo=on")?.value);
        assert_eq!(true, CommandLineOptionParser::parse_on_off("foo=ON")?.value);
        assert_eq!(
            false,
            CommandLineOptionParser::parse_on_off("foo=off")?.value
        );
        assert_eq!(
            false,
            CommandLineOptionParser::parse_on_off("foo=OFF")?.value
        );
        assert_eq!(
            true,
            CommandLineOptionParser::parse_on_off("foo=true").is_err()
        );

        Ok(())
    }
}
