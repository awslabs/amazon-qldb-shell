use crate::{settings::FormatMode, ui::Ui};
use amazon_qldb_driver::{ion_compat, transaction::StatementResults};
use anyhow::Result;
use ion_c_sys::reader::{IonCReaderHandle, IonCReader};
use ion_c_sys::result::{IonCError};
use ion_c_sys::{ION_TYPE_EOF};
use itertools::Itertools;
use std::collections::HashSet;

pub(crate) fn display_results(results: &StatementResults, format: &FormatMode, ui: &Box<dyn Ui>) {
    match format {
        FormatMode::Ion => display_results_ion_text(results, ui),
        FormatMode::Table => display_results_table(results, ui).unwrap(),
    }
}

fn display_results_ion_text(results: &StatementResults, ui: &Box<dyn Ui>) {
    let iter = results.readers().map(|r| ion_text_string(r));
    Itertools::intersperse(iter, ",\n".to_owned()).for_each(|p| ui.print(&p));
    ui.newline();
}

fn ion_text_string(result: Result<IonCReaderHandle, IonCError>) -> String {
    let value = match result {
        Ok(v) => v,
        Err(e) => {
            warn!(
                "unable to display document because it could not be parsed: {}",
                e
            );
            return String::new();
        }
    };

    match ion_compat::to_string_pretty(value) {
        Ok(d) => d,
        Err(e) => {
            warn!("ion formatter is not able to display this document: {}", e);
            return String::new();
        }
    }
}

fn display_results_table(results: &StatementResults, ui: &Box<dyn Ui>) -> Result<()> {
    let mut columns = HashSet::new();
    for reader in results.readers() {
        let mut reader = reader?;
        let _ = reader.next()?;
        let _ = reader.step_in()?;
        while reader.next()? != ION_TYPE_EOF {
            columns.insert(reader.get_field_name()?.as_str().to_string());
        }
    }
    ui.println(&format!("Your columns are: {:?}", &columns));
    return Ok(());
}
