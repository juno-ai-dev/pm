use std::{env, fs, path::PathBuf};

use cosmwasm_schema::schema_for;
use pm_types::PublicTypes;

fn main() {
    let schema = serde_json::to_string_pretty(&schema_for!(PublicTypes)).unwrap() + "\n";
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema/pm-types.json");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(output, schema).unwrap();
}
