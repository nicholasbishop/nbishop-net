use anyhow::Result;
use fs_err as fs;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use walkdir::WalkDir;

#[derive(Debug)]
struct Conf {
    content_dir: PathBuf,
    output_dir: PathBuf,
}

#[derive(Debug)]
struct Content {
    /// Input path with the first component being `Conf::content_dir`.
    source: PathBuf,

    /// Output path with the first component being `Conf::output_dir`.
    output: PathBuf,

    /// Output path relative to `Conf::output_dir`.
    rel_to_output_dir: PathBuf,
}

fn get_all_contents(conf: &Conf) -> Result<Vec<Content>> {
    let mut contents = Vec::new();

    for entry in WalkDir::new(&conf.content_dir) {
        let entry = entry?;

        if !entry.file_type().is_file() {
            continue;
        }

        // For now ignore anything but markdown files.
        if entry.path().extension().unwrap() != "md" {
            continue;
        }

        // Source path relative to the content dir.
        let rel_path = entry.path().strip_prefix(&conf.content_dir)?;

        // Create output subdirectory if needed.
        let output_dir = conf.output_dir.join(rel_path.parent().unwrap());

        let output_name = Path::new(entry.file_name()).with_extension("html");
        let output_path = output_dir.join(output_name);

        contents.push(Content {
            source: entry.path().into(),
            rel_to_output_dir: output_path
                .strip_prefix(&conf.output_dir)
                .unwrap()
                .into(),
            output: output_path,
        });
    }

    Ok(contents)
}

fn get_markdown_toc_list<P: AsRef<Path>>(
    contents: &[Content],
    prefix: P,
) -> String {
    contents
        .iter()
        .filter_map(|c| {
            if let Ok(rest) = c.rel_to_output_dir.strip_prefix(prefix.as_ref())
            {
                Some(format!("* {}", rest.display()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() -> Result<()> {
    let conf = Conf {
        content_dir: Path::new("content").into(),
        output_dir: Path::new("output").into(),
    };

    // Delete the output directory entirely before filling it.
    fs::remove_dir_all(&conf.output_dir)?;

    let tera = Tera::new("templates/**/*.html")?;

    let contents = get_all_contents(&conf)?;

    for content in &contents {
        let output_dir = content.output.parent().unwrap();
        if !output_dir.exists() {
            println!("mkdir {}", output_dir.display());
            fs::create_dir_all(&output_dir)?;
        }

        println!(
            "render {} -> {}",
            content.source.display(),
            content.output.display()
        );

        let mut markdown = fs::read_to_string(&content.source)?;

        // TODO: make more generic.
        let dir_notes_placeholder = "$$$ dir notes\n";
        if markdown.contains(dir_notes_placeholder) {
            let dir_notes = get_markdown_toc_list(&contents, "notes");
            markdown = markdown.replace(dir_notes_placeholder, &dir_notes);
        }

        let markdown_html =
            comrak::markdown_to_html(&markdown, &Default::default());

        let mut ctx = Context::new();
        ctx.insert("title", "todo!");
        ctx.insert("body", &markdown_html);
        let html = tera.render("base.html", &ctx)?;

        fs::write(&content.output, html)?;
    }

    Ok(())
}
