use anyhow::{anyhow, Result};
use fs_err as fs;
use std::collections::HashMap;
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

    /// Output file name.
    output_name: String,

    /// Input directory relative to `Conf::content_dir`.
    subdir: PathBuf,

    /// Front-matter map.
    front_matter: HashMap<String, String>,

    /// Everything after the front matter.
    body: String,
}

fn get_all_contents(conf: &Conf) -> Result<Vec<Content>> {
    let mut contents = Vec::new();

    for entry in WalkDir::new(&conf.content_dir) {
        let entry = entry?;
        let source = entry.path();

        if !entry.file_type().is_file() {
            continue;
        }

        // For now ignore anything but markdown files.
        if source.extension().unwrap() != "md" {
            continue;
        }

        // Source path relative to the content dir.
        let rel_path = source.strip_prefix(&conf.content_dir)?;

        let output_name = rel_path
            .with_extension("html")
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>()
            .join("_");

        // Read source and split out the front matter.
        let sep = "+++";
        let all = fs::read_to_string(source)?;
        let mut iter = all.splitn(3, sep).skip(1);
        let front = iter.next().ok_or_else(|| {
            anyhow!("missing front matter in {}", source.display())
        })?;
        let body = iter
            .next()
            .ok_or_else(|| anyhow!("missing body in {}", source.display()))?;
        let mut front_matter = HashMap::new();
        for line in front.lines() {
            let parts = line.splitn(2, ':').collect::<Vec<_>>();
            if parts.len() == 2 {
                front_matter.insert(
                    parts[0].trim().to_owned(),
                    parts[1].trim().to_owned(),
                );
            }
        }

        contents.push(Content {
            source: source.into(),
            output_name,
            subdir: rel_path.parent().unwrap().into(),
            front_matter,
            body: body.into(),
        });
    }

    Ok(contents)
}

fn get_markdown_toc_list<P: AsRef<Path>>(
    contents: &[Content],
    subdir: P,
) -> String {
    contents
        .iter()
        .filter_map(|c| {
            if c.subdir == subdir.as_ref() {
                let title = &c.front_matter["title"];
                let date = &c.front_matter["date"];
                Some(format!("* {} - [{}]({})", date, title, c.output_name))
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
    if conf.output_dir.exists() {
        fs::remove_dir_all(&conf.output_dir)?;
    }
    fs::create_dir(&conf.output_dir)?;

    let tera = Tera::new("templates/**/*.html")?;

    let contents = get_all_contents(&conf)?;

    for content in &contents {
        let output_path = conf.output_dir.join(&content.output_name);

        println!(
            "render {} -> {}",
            content.source.display(),
            output_path.display()
        );

        let mut markdown = content.body.clone();

        // TODO: make more generic.
        let dir_notes_placeholder = "$$$ dir notes\n";
        if markdown.contains(dir_notes_placeholder) {
            let dir_notes = get_markdown_toc_list(&contents, "notes");
            markdown = markdown.replace(dir_notes_placeholder, &dir_notes);
        }

        // Prefix with title.
        let title = &content.front_matter["title"];
        markdown = format!("# {}\n{}", title, markdown);

        let markdown_html =
            comrak::markdown_to_html(&markdown, &Default::default());

        let mut ctx = Context::new();
        ctx.insert("title", title);
        ctx.insert("body", &markdown_html);
        let html = tera.render("base.html", &ctx)?;

        fs::write(&output_path, html)?;
    }

    fs::copy("css/style.css", conf.output_dir.join("style.css"))?;

    Ok(())
}
