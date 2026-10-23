// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Input and output utilities for the library, such as writing text to files or managing folders.

use std::fs;

use std::io::Write;

/// Writing of data to CSV files
pub mod csv_data;
/// Management of folders
pub mod folder_management;

/// Writes the text to a file. The file is created if it does not exist, and overwritten if it does.
pub fn write_text_to_file(file_path: &str, text: &str) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;

    file.write_all(text.as_bytes())?;
    Ok(())
}
