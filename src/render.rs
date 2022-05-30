use anyhow::{anyhow, bail, Result};
use camino::{Utf8Path, Utf8PathBuf};
use comrak::plugins::syntect::SyntectAdapter;
use comrak::{ComrakOptions, ComrakPlugins, ComrakRenderOptions};
use fs_err as fs;
use std::collections::HashMap;
use std::process::Command;
use tera::{Context, Tera};
use time::OffsetDateTime;
use walkdir::WalkDir;

#[derive(Debug)]
struct Conf {
    content_dir: Utf8PathBuf,
    output_dir: Utf8PathBuf,
}

#[derive(Debug)]
struct FrontMatter {
    title: String,
    date: Option<String>,
}

#[derive(Debug)]
struct MarkdownContent {
    /// Front-matter.
    front_matter: FrontMatter,

    /// Everything after the front matter.
    body: String,
}

#[derive(Debug)]
enum ContentType {
    /// Markdown content that must be rendered.
    Markdown(MarkdownContent),

    /// Regular file that can just be copied to the output.
    PlainFile,
}

#[derive(Debug)]
struct Content {
    /// Input path with the first component being `Conf::content_dir`.
    source: Utf8PathBuf,

    /// Date and time when the input was last modified. This uses the
    /// git commit date.
    last_modified: OffsetDateTime,

    /// Output file name.
    output_name: String,

    /// Input directory relative to `Conf::content_dir`.
    subdir: Utf8PathBuf,

    content_type: ContentType,
}

fn get_markdown_content(source: &Utf8Path) -> Result<MarkdownContent> {
    // Read source and split out the front matter.
    let sep = "+++";
    let all = fs::read_to_string(source)?;
    let mut iter = all.splitn(3, sep).skip(1);
    let front = iter
        .next()
        .ok_or_else(|| anyhow!("missing front matter in {}", source))?;
    let body = iter
        .next()
        .ok_or_else(|| anyhow!("missing body in {}", source))?;
    let mut front_matter = HashMap::new();
    for line in front.lines() {
        let parts = line.splitn(2, ':').collect::<Vec<_>>();
        if parts.len() == 2 {
            front_matter.insert(parts[0].trim(), parts[1].trim());
        }
    }
    let front_matter = FrontMatter {
        title: front_matter["title"].to_owned(),
        date: front_matter.get("date").map(|s| s.to_owned().to_owned()),
    };

    Ok(MarkdownContent {
        front_matter,
        body: body.into(),
    })
}

fn get_last_modified(path: &Utf8Path) -> Result<OffsetDateTime> {
    // "%ct" is the committer date formatted in unix time.
    let output = Command::new("git")
        .args(&["log", "-1", "--format=format:%ct"])
        .arg(path)
        .output()?;
    if !output.status.success() {
        bail!("failed to get date of {}: {:?}", path, output);
    }
    let s = std::str::from_utf8(&output.stdout).unwrap();
    let seconds: i64 = s.parse().unwrap();
    Ok(OffsetDateTime::from_unix_timestamp(seconds)?)
}

fn get_all_contents(conf: &Conf) -> Result<Vec<Content>> {
    let mut contents = Vec::new();

    for entry in WalkDir::new(&conf.content_dir) {
        let entry = entry?;
        let source = Utf8Path::from_path(entry.path()).ok_or_else(|| {
            anyhow!("path is not UTF-8: {}", entry.path().display())
        })?;

        if !entry.file_type().is_file() {
            continue;
        }

        // Source path relative to the content dir.
        let rel_path = source.strip_prefix(&conf.content_dir)?;

        let mut output_name = rel_path.iter().collect::<Vec<_>>().join("-");

        let extension = source.extension().unwrap();
        let content_type;
        let plain_file_extensions = ["css", "jpg", "png"];
        if extension == "md" {
            // TODO: could be more precise with this.
            output_name = output_name.replacen(".md", ".html", 1);

            content_type = ContentType::Markdown(get_markdown_content(source)?);
        } else if plain_file_extensions.contains(&extension) {
            content_type = ContentType::PlainFile;
        } else {
            println!("ignoring {}", source);
            continue;
        }

        contents.push(Content {
            source: source.into(),
            last_modified: get_last_modified(source)?,
            output_name,
            subdir: rel_path.parent().unwrap().into(),
            content_type,
        });
    }

    // Sort by (date, name).
    contents.sort_unstable_by_key(|c| {
        let date = if let ContentType::Markdown(md) = &c.content_type {
            md.front_matter.date.clone()
        } else {
            None
        };
        (date, c.output_name.clone())
    });

    Ok(contents)
}

fn get_markdown_toc_list<P: AsRef<Utf8Path>>(
    contents: &[Content],
    subdir: P,
) -> String {
    contents
        .iter()
        // Reverse iteration so that newer entries come first.
        .rev()
        .filter_map(|c| {
            let md = if let ContentType::Markdown(md) = &c.content_type {
                md
            } else {
                return None;
            };

            if c.subdir == subdir.as_ref() {
                let title = &md.front_matter.title;
                let date = &md.front_matter.date.as_ref().unwrap();
                Some(format!("* {} - [{}]({})", date, title, c.output_name))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct RenderMarkdownState<'a> {
    contents: &'a [Content],
    content: &'a Content,
    md: &'a MarkdownContent,
    log_toc: &'a str,
    notes_toc: &'a str,
    options: &'a ComrakOptions,
    plugins: &'a ComrakPlugins<'a>,
    tera: &'a Tera,
    output_path: &'a Utf8Path,
}

fn render_markdown(state: RenderMarkdownState) -> Result<()> {
    let markdown = state.md.body.clone();

    let mut ctx = Context::new();
    ctx.insert("log_toc", state.log_toc);
    ctx.insert("notes_toc", state.notes_toc);
    let autoescape = true;
    let markdown = Tera::one_off(&markdown, &ctx, autoescape)?;

    let markdown_html = comrak::markdown_to_html_with_plugins(
        &markdown,
        state.options,
        state.plugins,
    );

    let mut ctx = Context::new();
    ctx.insert("title", &state.md.front_matter.title);
    ctx.insert(
        "created_date",
        &state
            .md
            .front_matter
            .date
            .as_ref()
            .unwrap_or(&"?".to_string()),
    );
    ctx.insert(
        "updated_date",
        &state.content.last_modified.date().to_string(),
    );
    ctx.insert("body", &markdown_html);
    let show_home_link = state.content.output_name != "index.html";
    ctx.insert("show_home_link", &show_home_link);
    let html = state.tera.render("base.html", &ctx)?;

    fs::write(&state.output_path, html)?;

    Ok(())
}

pub fn render() -> Result<()> {
    let conf = Conf {
        content_dir: "content".into(),
        output_dir: "output".into(),
    };

    // Delete the output directory entirely before filling it.
    if conf.output_dir.exists() {
        fs::remove_dir_all(&conf.output_dir)?;
    }
    fs::create_dir(&conf.output_dir)?;

    // Create code-highlighting plugin.
    let adapter = SyntectAdapter::new("base16-ocean.light");
    let options = ComrakOptions {
        render: ComrakRenderOptions {
            unsafe_: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut plugins = ComrakPlugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    // Load templates.
    let tera = Tera::new("templates/**/*.html")?;

    let contents = get_all_contents(&conf)?;

    let log_toc = get_markdown_toc_list(&contents, "log");
    let notes_toc = get_markdown_toc_list(&contents, "notes");

    for content in &contents {
        let output_path = conf.output_dir.join(&content.output_name);

        if let ContentType::Markdown(md) = &content.content_type {
            println!("render {} -> {}", content.source, output_path);

            render_markdown(RenderMarkdownState {
                contents: &contents,
                log_toc: &log_toc,
                notes_toc: &notes_toc,
                content,
                md,
                options: &options,
                plugins: &plugins,
                tera: &tera,
                output_path: &output_path,
            })?;
        } else {
            println!("copy {} -> {}", content.source, output_path);
            fs::copy(&content.source, &output_path)?;
        }
    }

    Ok(())
}
