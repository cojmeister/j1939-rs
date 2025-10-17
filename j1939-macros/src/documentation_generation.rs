use crate::field_info::{format_binary_value, Encoding, EnumVariant, FieldInfo};
use tabled::settings::Style;
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct DataField {
    bit_offset: usize,
    bit_length: usize,
    description: String,
    data_type: String,
    units: String,
}


pub fn generate_documentation_table(fields: &Vec<FieldInfo>) -> String {
    let data_fields: Vec<DataField> = fields.iter().map(|f| {
        DataField {
            bit_offset: f.bit_start,
            bit_length: f.bit_length(),
            description: f.doc.join(" - "),
            data_type: f.encoding.to_string(),
            units: f.units.clone().unwrap_or("".to_string()),
        }
    }).collect();

    let mut output = String::new();

    // Generate main field table
    let mut table = Table::new(data_fields);
    table.with(Style::markdown());
    output.push_str(&table.to_string());

    // Generate enum documentation sections
    for field in fields {
        if let Encoding::Enum(enum_type) = &field.encoding {
            output.push_str("\n\n");
            output.push_str(&generate_enum_documentation(field, enum_type));
        }
    }

    output
}

#[derive(Tabled)]
struct EnumTable {
    value: usize,
    binary: String,
    name: String,
    description: String,
}

fn generate_enum_documentation(field: &FieldInfo, enum_type: &syn::Type) -> String {
    let mut output = String::new();
    let enum_name = quote::quote!(#enum_type).to_string();

    output.push_str(&format!("### {} Values\n\n", enum_name));

    // Extract enum variants and enum-level documentation
    let (enum_docs, variants) = extract_enum_variants(&enum_name);
    let bit_width = field.bit_length();

    // Add enum-level documentation if present
    if !enum_docs.is_empty() {
        for doc_line in &enum_docs {
            if !doc_line.is_empty() {
                output.push_str(&format!("{}\n", doc_line));
            }
        }
        output.push('\n'); // Add blank line after enum description
    }

    // Add the variant table
    let table_rows: Vec<EnumTable> = variants.into_iter().map(|v| {
        let description = if v.doc_comments.is_empty() {
            String::new()
        } else {
            v.doc_comments.join(" ")
        };
        EnumTable {
            value: v.value as usize,
            binary: format_binary_value(v.value, bit_width),
            name: v.name,
            description,
        }
    }).collect();

    // Generate the variant table
    let mut table = Table::new(table_rows);
    table.with(Style::markdown());
    output.push_str(&table.to_string());

    output
}

fn extract_enum_variants(enum_name: &str) -> (Vec<String>, Vec<EnumVariant>) {
    // Try to get enum information from the registry
    if let Some(enum_info) = crate::field_info::get_enum_info(enum_name) {
        (enum_info.doc_comments, enum_info.variants)
    } else {
        // Fallback for unregistered enums
        (
            vec![format!("Enum '{}' not registered with #[j1939_enum]", enum_name)],
            vec![
                EnumVariant {
                    name: "Unknown".to_string(),
                    value: 0,
                    doc_comments: vec!["Use #[j1939_enum] attribute on the enum definition".to_string()],
                }
            ]
        )
    }
}