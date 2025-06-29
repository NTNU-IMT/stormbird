use std::fs;
use std::path::Path;

pub fn ensure_folder_exists(folder_path: &str) -> std::io::Result<()> {
    let path = Path::new(folder_path);
    
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}