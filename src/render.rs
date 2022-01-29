use anyhow::{anyhow, Result};
use camino::{Utf8Path, Utf8PathBuf};
use comrak::plugins::syntect::SyntectAdapter;
use comrak::{ComrakOptions, ComrakPlugins, ComrakRenderOptions};
use fs_err as fs;
use std::collections::HashMap;
use tera::{Context, Tera};
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
struct Content {
    /// Input path with the first component being `Conf::content_dir`.
    source: Utf8PathBuf,

    /// Output file name.
    output_name: String,

    /// Input directory relative to `Conf::content_dir`.
    subdir: Utf8PathBuf,

    /// Front-matter.
    front_matter: FrontMatter,

    /// Everything after the front matter.
    body: String,
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

        // For now ignore anything but markdown files.
        if source.extension().unwrap() != "md" {
            continue;
        }

        // Source path relative to the content dir.
        let rel_path = source.strip_prefix(&conf.content_dir)?;

        let output_name = rel_path
            .with_extension("html")
            .iter()
            .collect::<Vec<_>>()
            .join("_");

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

fn get_markdown_toc_list<P: AsRef<Utf8Path>>(
    contents: &[Content],
    subdir: P,
) -> String {
    contents
        .iter()
        .filter_map(|c| {
            if c.subdir == subdir.as_ref() {
                let title = &c.front_matter.title;
                let date = &c.front_matter.date.as_ref().unwrap();
                Some(format!("* {} - [{}]({})", date, title, c.output_name))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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

    for content in &contents {
        let output_path = conf.output_dir.join(&content.output_name);

        println!("render {} -> {}", content.source, output_path);

        let mut markdown = content.body.clone();

        // TODO: make more generic.
        let dir_names = ["log", "notes"];
        for name in dir_names {
            let placeholder = format!("$$$ dir {}\n", name);
            if markdown.contains(&placeholder) {
                let toc = get_markdown_toc_list(&contents, name);
                markdown = markdown.replace(&placeholder, &toc);
            }
        }

        let show_home_link = content.output_name != "index.html";
        let markdown_html = comrak::markdown_to_html_with_plugins(
            &markdown, &options, &plugins,
        );

        let mut ctx = Context::new();
        ctx.insert("title", &content.front_matter.title);
        ctx.insert("body", &markdown_html);
        ctx.insert("show_home_link", &show_home_link);
        let html = tera.render("base.html", &ctx)?;

        fs::write(&output_path, html)?;
    }

    let extra_sources = [
        "content/favicon.png",
        "content/h1.png",
        "content/sfc.png",
        "css/style.css",
    ];
    for src in extra_sources {
        fs::copy(
            src,
            conf.output_dir
                .join(Utf8Path::new(src).file_name().unwrap()),
        )?;
    }

    Ok(())
}
