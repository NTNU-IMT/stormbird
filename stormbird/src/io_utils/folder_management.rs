// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use std::fs;
use std::path::Path;

pub fn ensure_folder_exists(folder_path: &Path) -> std::io::Result<()> {    
    if !folder_path.exists() {
        fs::create_dir_all(folder_path)?;
    }
    Ok(())
}