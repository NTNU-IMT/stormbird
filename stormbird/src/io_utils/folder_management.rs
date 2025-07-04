use std::fs;
use std::path::Path;

pub fn ensure_folder_exists(folder_path: &Path) -> std::io::Result<()> {    
    if !folder_path.exists() {
        fs::create_dir_all(folder_path)?;
    }
    Ok(())
}