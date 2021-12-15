use anyhow::Result;
use fs_err as fs;
use std::path::Path;
use tera::{Context, Tera};
use walkdir::WalkDir;

fn main() -> Result<()> {
    let content_dir = Path::new("content");
    let output_dir = Path::new("output");

    // Delete the output directory entirely before filling it.
    fs::remove_dir_all(output_dir)?;

    let tera = Tera::new("templates/**/*.html")?;

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

        let extension = entry.path().extension().unwrap();

        if extension == "md" {
            let output_name = Path::new(entry.file_name()).with_extension("html");
            let output_path = output_dir.join(output_name);

            println!(
                "render {} -> {}",
                entry.path().display(),
                output_path.display()
            );

            let markdown = fs::read_to_string(entry.path())?;
            let markdown_html = comrak::markdown_to_html(&markdown, &Default::default());

            let mut ctx = Context::new();
            ctx.insert("title", "todo!");
            ctx.insert("body", &markdown_html);
            let html = tera.render("base.html", &ctx)?;

            fs::write(&output_path, html)?;
        }
    }

    Ok(())
}
