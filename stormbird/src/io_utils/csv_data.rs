// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::path::Path;
use std::fs;
use std::io::Write;

use crate::error::Error;

pub fn create_or_append_header_and_data_strings_file(
    file_path_str: &str,
    header: &str,
    data: &str,
) -> Result<(), Error> {

    let file_path = Path::new(file_path_str);

    if file_path.exists() {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(file_path)?;

        writeln!(file, "{}", data)?;
    } else {
        let mut file = fs::File::create(file_path)?;

        writeln!(file, "{}", header)?;
        writeln!(file, "{}", data)?;
    }

    Ok(())
}
