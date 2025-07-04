
use std::fs;

use std::io::Write;

pub mod csv_data;
pub mod folder_management;

pub fn write_text_to_file(file_path: &str, text: &str) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;

    file.write_all(text.as_bytes())?;
    Ok(())
}
