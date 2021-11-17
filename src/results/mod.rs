use crate::{settings::FormatMode, ui::Ui};
use amazon_qldb_driver::ion_compat;
use amazon_qldb_driver::results::BufferedStatementResults;
use anyhow::Result;
use ion_c_sys::reader::IonCReaderHandle;
use ion_c_sys::result::IonCError;
use itertools::Itertools;
use table::display_results_table;
use tracing::warn;

mod table;

pub(crate) fn display_results(
    results: &BufferedStatementResults,
    format: &FormatMode,
    ui: &Box<dyn Ui>,
) {
    match format {
        FormatMode::Ion => display_results_ion_text(results, ui),
        FormatMode::Table => {
            if let Err(e) = display_results_table(results, ui) {
                ui.warn(&format!("unable to print results: {}", e));
            }
        }
    }
}

fn display_results_ion_text(results: &BufferedStatementResults, ui: &Box<dyn Ui>) {
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
