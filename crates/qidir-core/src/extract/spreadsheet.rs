//! Spreadsheets (XLSX, XLSM, XLSB, XLS, ODS) via `calamine`.

use std::io::Cursor;

use calamine::{Data, Reader, Sheets};

use crate::error::{Error, Result};

/// Maximum number of cells converted to text per workbook (protects against
/// gigantic exports).
const MAX_CELLS: usize = 2_000_000;

/// Extract all sheet names and cell values row by row, tab separated.
pub fn spreadsheet(bytes: &[u8], ext: &str) -> Result<String> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut wb: Sheets<_> = match ext {
        "xlsx" | "xlsm" | "xltx" | "xltm" => {
            Sheets::Xlsx(calamine::Xlsx::new(cursor).map_err(|e| Error::Extract(e.to_string()))?)
        }
        "xlsb" => {
            Sheets::Xlsb(calamine::Xlsb::new(cursor).map_err(|e| Error::Extract(e.to_string()))?)
        }
        "xls" | "xlt" => {
            Sheets::Xls(calamine::Xls::new(cursor).map_err(|e| Error::Extract(e.to_string()))?)
        }
        "ods" | "ots" => {
            Sheets::Ods(calamine::Ods::new(cursor).map_err(|e| Error::Extract(e.to_string()))?)
        }
        _ => return Err(Error::Unsupported(ext.to_string())),
    };

    let names: Vec<String> = wb.sheet_names().to_vec();
    let mut out = String::new();
    let mut cells = 0usize;
    for name in names {
        let range = match wb.worksheet_range(&name) {
            Ok(r) => r,
            Err(_) => continue,
        };
        out.push_str(&name);
        out.push('\n');
        for row in range.rows() {
            let mut line = String::new();
            let mut first = true;
            for cell in row {
                cells += 1;
                if cells > MAX_CELLS {
                    break;
                }
                let s = cell_to_string(cell);
                if s.is_empty() {
                    continue;
                }
                if !first {
                    line.push('\t');
                }
                first = false;
                line.push_str(&s);
            }
            if !line.is_empty() {
                out.push_str(&line);
                out.push('\n');
            }
            if cells > MAX_CELLS {
                break;
            }
        }
        out.push('\n');
    }
    Ok(out.trim().to_string())
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.trim().to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => excel_datetime_to_string(d.as_f64()),
        Data::Error(e) => format!("{e:?}"),
    }
}

/// Convert an Excel serial date (days since 1899-12-30) to `YYYY-MM-DD[ HH:MM]`.
fn excel_datetime_to_string(serial: f64) -> String {
    if !serial.is_finite() || !(0.0..=2_958_465.0).contains(&serial) {
        return format!("{serial}");
    }
    let days = serial.floor() as i64;
    let secs = ((serial - serial.floor()) * 86_400.0).round() as i64;
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
    let date = match base.checked_add_signed(chrono::Duration::days(days)) {
        Some(d) => d,
        None => return format!("{serial}"),
    };
    if secs == 0 {
        date.format("%Y-%m-%d").to_string()
    } else {
        let t =
            chrono::NaiveTime::from_num_seconds_from_midnight_opt(secs as u32 % 86_400, 0).unwrap();
        chrono::NaiveDateTime::new(date, t)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::ooxml::build_zip;

    #[test]
    fn xlsx_with_shared_strings() {
        let content_types = r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/></Types>"#;
        let rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
        let workbook = r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Продажи" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
        let wb_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/></Relationships>"#;
        let sst = r#"<?xml version="1.0" encoding="UTF-8"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2" uniqueCount="2"><si><t>Товар</t></si><si><t>Алматы</t></si></sst>"#;
        let sheet = r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"><v>1500</v></c></row><row r="2"><c r="A2" t="s"><v>1</v></c><c r="B2"><v>2.5</v></c></row></sheetData></worksheet>"#;
        let bytes = build_zip(&[
            ("[Content_Types].xml", content_types),
            ("_rels/.rels", rels),
            ("xl/workbook.xml", workbook),
            ("xl/_rels/workbook.xml.rels", wb_rels),
            ("xl/sharedStrings.xml", sst),
            ("xl/worksheets/sheet1.xml", sheet),
        ]);
        let t = spreadsheet(&bytes, "xlsx").unwrap();
        assert_eq!(t, "Продажи\nТовар\t1500\nАлматы\t2.5");
    }
}
