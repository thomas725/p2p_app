// https://docs.rs/diesel_migrations/latest/diesel_migrations/macro.embed_migrations.html
// Due to limitations in rusts proc-macro API there is currently no way to signal that a
// specific proc macro should be rerun if some external file changes/is added. This implies
// that embed_migrations! cannot regenerate the list of embedded migrations if only the
// migrations are changed. This limitation can be solved by adding a custom build.rs file
// to your crate, such that the crate is rebuild if the migration directory changes.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=src/generated/schema.rs");

    if let Err(e) = parse_schema_rs() {
        eprintln!("parse_schema_rs error: {}", e);
        return Err(e);
    }

    Ok(())
}

fn parse_schema_rs() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let schema_path = Path::new(&manifest_dir).join("src/generated/schema.rs");
    let schema_content = fs::read_to_string(&schema_path)?;

    let mut entries: Vec<(String, String, String)> = Vec::new();
    let mut lines: Vec<&str> = schema_content.lines().collect();
    lines.push("");

    let mut i = 0;
    let mut table_name = String::new();
    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("diesel::table!") || line.starts_with("diesel::table! {") {
            table_name.clear();
            i += 1;
            while i < lines.len() {
                let inner_line = lines[i].trim();
                if inner_line.contains("(id)") {
                    let part = inner_line.split('(').next().unwrap_or("");
                    table_name = part.trim().to_string();
                    break;
                }
                i += 1;
            }
        }

        if !table_name.is_empty() && line.contains("->") {
            let parts: Vec<&str> = line.split("->").collect();
            if parts.len() == 2 {
                let col_name = parts[0].trim();
                let type_part = parts[1].trim().trim_end_matches(',');
                let sql_type = map_sql_type(type_part);
                let tn = table_name.clone();
                entries.push((tn, col_name.to_string(), sql_type.to_string()));
            }
        }

        if line == "}" && !table_name.is_empty() {
            table_name.clear();
        }

        i += 1;
    }

    entries.sort_by(|a, b| {
        let t = a.0.cmp(&b.0);
        if t == std::cmp::Ordering::Equal {
            a.1.cmp(&b.1)
        } else {
            t
        }
    });

    let src_dir = Path::new(&manifest_dir).join("src");
    let dest_path = src_dir.join("generated/columns.rs");
    let mut output = String::from("pub const SCHEMA_ENTRIES: &[(&str, &str, &str)] = &[\n");
    for (i, entry) in entries.iter().enumerate() {
        let escaped_table = entry.0.replace('"', "\\\"");
        let escaped_col = entry.1.replace('"', "\\\"");
        let escaped_type = entry.2.replace('"', "\\\"");
        let suffix = if i + 1 == entries.len() { "\n" } else { ",\n" };
        output.push_str(&format!(
            "    (\"{}\", \"{}\", \"{}\"){}",
            escaped_table, escaped_col, escaped_type, suffix
        ));
    }
    output.push_str("];\n");

    fs::write(dest_path, output)?;

    Ok(())
}

fn map_sql_type(diesel_type: &str) -> &'static str {
    match diesel_type {
        "Integer" => "INTEGER",
        "Timestamp" => "TIMESTAMP",
        "Binary" => "BLOB",
        "Text" => "TEXT",
        "Bool" => "INTEGER",
        _ if diesel_type.starts_with("Nullable<") => "TEXT",
        _ => "TEXT",
    }
}
