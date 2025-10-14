use crate::field_info::FieldInfo;
use tabled::{Table, Tabled};
use tabled::settings::Style;

#[derive(Tabled)]
struct DataField {
    bit_offset: usize,
    bit_length: usize,
    description: String,
    data_type: String,
    units: String,
}


pub fn generate_documentation_table(fields: &Vec<FieldInfo>) -> String {
    let data_fields: Vec<DataField> = fields.into_iter().map(|f| {
        DataField {
            bit_offset: f.bit_start,
            bit_length: f.bit_length(),
            description: f.doc.join(" - "),
            data_type: f.encoding.to_string(),
            units: f.units.clone().unwrap_or("".to_string()),
        }
    }).collect();


    let mut table = Table::new(data_fields);
    table.with(Style::markdown());
    table.to_string()

}