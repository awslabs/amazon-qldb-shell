use crate::ui::Ui;
use amazon_qldb_driver::transaction::StatementResults;
use anyhow::{anyhow, Result};
use comfy_table::Table;
use ion_rs::value::*;
use ion_rs::value::{
    loader::{loader, Loader},
    owned::OwnedElement,
};
use ion_rs::IonType;
use std::collections::HashSet;

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

    for elem in elems {
        let strukt = match elem.ion_type() {
            IonType::Struct => elem.as_struct().unwrap(),
            _ => Err(anyhow!("value types are not yet supported"))?,
        };

        for (field, _) in strukt.iter() {
            let heading = field.text().unwrap().to_string();
            if headers_set.insert(heading.clone()) {
                headers.push(heading);
            }
        }
    }

    // not really needed, but makes it clear that this set has done it's job.
    drop(headers_set);

    let mut table = Table::new();
    table.set_header(headers.clone());

    for elem in elems {
        let strukt = match elem.ion_type() {
            IonType::Struct => elem.as_struct().unwrap(),
            _ => Err(anyhow!("value types are not yet supported"))?,
        };

        let mut row = vec![];
        for field in &headers {
            row.push(format_element_for_cell(strukt.get(field))?);
        }
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
        IonType::Decimal => todo!("upstream to_string()"),
        IonType::Timestamp => todo!("upstream to_string()"),
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
