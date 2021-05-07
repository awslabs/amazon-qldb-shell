use crate::ui::Ui;
use amazon_qldb_driver::transaction::StatementResults;
use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use comfy_table::Table;
use ion_rs::value::*;
use ion_rs::value::{
    loader::{loader, Loader},
    owned::OwnedElement,
};
use ion_rs::IonType;
use std::collections::HashSet;
use std::convert::TryFrom;

pub(crate) fn display_results_table(results: &StatementResults, ui: &Box<dyn Ui>) -> Result<()> {
    let loader = loader();

    let elems: Vec<_> = results
        .raw_values()
        .map(|data| match loader.iterate_over(data)?.next() {
            None => Err(anyhow!("found no value, which is unexpected"))?,
            Some(r) => Ok(r?),
        })
        .collect::<Result<_>>()?;

    let refs: Vec<_> = elems.iter().map(|e| e).collect();
    let table = format_table(&refs[..])?;
    ui.println(&format!("{}", table));

    Ok(())
}

fn format_table(elems: &[&OwnedElement]) -> Result<String> {
    Ok(if elems.len() > 0 {
        format!("{}", build_table(elems)?)
    } else {
        "".to_string()
    })
}

fn build_table(elems: &[&OwnedElement]) -> Result<Table> {
    let mut headers_set = HashSet::new();
    let mut headers = vec![];
    let mut single_value = false;

    for elem in elems {
        match elem.ion_type() {
            IonType::Struct => {
                let strukt = elem.as_struct().unwrap();

                for (field, _) in strukt.iter() {
                    let heading = field.text().unwrap().to_string();
                    if headers_set.insert(heading.clone()) {
                        headers.push(heading);
                    }
                }
            }
            _ => single_value = true,
        };
    }

    // not really needed, but makes it clear that this set has done it's job.
    drop(headers_set);
    let mut final_headers = vec![];
    // If we found a single value, push in a special header.
    //
    // Note that top-level QLDB documents can only be structs, so at the
    // top-level, the only way to get values back is with `select value $field
    // from $table`.
    //
    // However, this code theoretically can handle mixed values. There is one
    // weird case where 1 result is a value while another has a field called
    // VALUE. In that case, the column VALUE will appear twice.
    if single_value {
        final_headers.push("VALUE".to_string());
    }
    final_headers.extend(headers);

    let mut table = Table::new();
    table.set_header(final_headers.clone());

    for elem in elems {
        let row = match elem.ion_type() {
            IonType::Struct => {
                let strukt = elem.as_struct().unwrap();

                let mut row = vec![];
                for field in &final_headers {
                    row.push(format_element_for_cell(strukt.get(field))?);
                }
                row
            }
            _ => {
                let mut row = vec!["".to_string(); final_headers.len()];
                row[0] = format_element_for_cell(Some(elem))?;
                row
            }
        };

        table.add_row(row);
    }

    Ok(table)
}

fn format_element_for_cell(elem: Option<&OwnedElement>) -> Result<String> {
    let elem = match elem {
        None => return Ok("".to_string()),
        Some(e) => e,
    };

    Ok(match elem.ion_type() {
        IonType::Null => "null".to_string(),
        IonType::Boolean => elem.as_bool().unwrap().to_string(),
        IonType::Integer => match elem.as_any_int().unwrap() {
            AnyInt::I64(i) => i.to_string(),
            AnyInt::BigInt(i) => i.to_string(),
        },
        IonType::Float => elem.as_f64().unwrap().to_string(),
        IonType::Decimal => {
            let decimal = elem.as_decimal().unwrap();
            match BigDecimal::try_from(decimal.clone()) {
                Ok(big) => format!("{}", big),
                Err(()) => format!("-0"),
            }
        }
        IonType::Timestamp => Err(anyhow!("timestamps are not yet supported"))?,
        IonType::Symbol => elem.as_sym().unwrap().text().unwrap().to_string(),
        IonType::String => elem.as_str().unwrap().to_string(),
        IonType::Clob | IonType::Blob => {
            let bytes = elem.as_bytes().unwrap();
            format!("{} bytes", bytes.len())
        }
        IonType::List | IonType::SExpression => {
            let seq = elem.as_sequence().unwrap();
            let elems: Vec<_> = seq.iter().collect();
            format_table(&elems[..])?
        }
        IonType::Struct => format_table(&[elem])?,
    })
}
