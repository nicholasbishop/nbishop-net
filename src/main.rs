use anyhow::Result;
use fs_err as fs;
use std::path::Path;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let content_dir = Path::new("content");
    let output_dir = Path::new("output");

    for entry in WalkDir::new(content_dir) {
        let entry = entry?;

        if !entry.file_type().is_file() {
            continue;
        }

        // Source path relative to the content dir.
        let rel_path = entry.path().strip_prefix(content_dir)?;

        // Create output subdirectory if needed.
        let output_dir = output_dir.join(rel_path.parent().unwrap());
        if !output_dir.exists() {
            println!("mkdir {}", output_dir.display());
            fs::create_dir_all(&output_dir)?;
        }

        // TODO: for now just copy stuff
        let dst_path = output_dir.join(entry.file_name());
        println!("cp {} {}", entry.path().display(), dst_path.display());
        fs::copy(entry.path(), dst_path)?;

        dbg!(output_dir);
    }

    // Recursively render everything in the content directory.
    // Output into the output directory.

    Ok(())
}
