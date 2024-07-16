use anyhow::{anyhow, bail, Context as _, Result};
use camino::{Utf8Path, Utf8PathBuf};
use comrak::plugins::syntect::SyntectAdapter;
use comrak::{ComrakOptions, ComrakPlugins, ComrakRenderOptions};
use fs_err as fs;
use image::imageops::{self, FilterType};
use image::io::Reader as ImageReader;
use rayon::prelude::*;
use rss::ChannelBuilder;
use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;
use tera::{Context, Tera};
use time::format_description::well_known::Rfc2822;
use time::{Date, OffsetDateTime};
use walkdir::WalkDir;

#[derive(Debug)]
struct Conf {
    content_dir: Utf8PathBuf,
    output_dir: Utf8PathBuf,
}

#[derive(Debug)]
struct FrontMatter {
    title: String,
    date: Option<Date>,
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

    /// Large photo that needs a thumbnail generated.
    Photo,

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

    let date_format = time::format_description::parse("[year]-[month]-[day]")?;
    let date = front_matter
        .get("date")
        .map(|date| Date::parse(date, &date_format).unwrap());

    let front_matter = FrontMatter {
        title: front_matter["title"].to_owned(),
        date,
    };

    Ok(MarkdownContent {
        front_matter,
        body: body.into(),
    })
}

fn get_last_modified(path: &Utf8Path) -> Result<OffsetDateTime> {
    // "%ct" is the committer date formatted in unix time.
    let output = Command::new("git")
        .args(["log", "-1", "--format=format:%ct"])
        .arg(path)
        .output()?;
    if !output.status.success() {
        bail!("failed to get date of {}: {:?}", path, output);
    }
    let s = std::str::from_utf8(&output.stdout).unwrap();
    let seconds: i64 = s
        .parse()
        .context(format!("failed to get last-modified-time for {path}"))?;
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
        let plain_file_extensions = ["css", "png", "svg"];
        if extension == "md" {
            // TODO: could be more precise with this.
            output_name = output_name.replacen(".md", ".html", 1);

            content_type = ContentType::Markdown(get_markdown_content(source)?);
        } else if extension == "jpg" {
            content_type = ContentType::Photo;
        } else if plain_file_extensions.contains(&extension) {
            content_type = ContentType::PlainFile;
        } else {
            println!("ignoring {source}");
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
            md.front_matter.date
        } else {
            None
        };
        (date, c.output_name.clone())
    });

    Ok(contents)
}

fn generate_rss(conf: &Conf, contents: &[Content]) -> Result<()> {
    let output_path = conf.output_dir.join("feed.rss");
    println!("generating rss -> {output_path}");

    let base_url = "http://nbishop.net";

    let build_date = OffsetDateTime::now_utc().format(&Rfc2822)?;

    let mut builder = ChannelBuilder::default();
    builder
        .title("nbishop.net")
        .link(base_url)
        .last_build_date(build_date.clone())
        .pub_date(build_date)
        .description("Nicholas Bishop's personal website");

    // Newest first.
    for content in contents.iter().rev() {
        if let ContentType::Markdown(md) = &content.content_type {
            // Anything that doesn't have a creation date (i.e. an
            // index) doesn't need to be in the feed.
            if md.front_matter.date.is_none() {
                continue;
            }

            let last_modified = content.last_modified.format(&Rfc2822)?;

            let item = rss::Item {
                title: Some(md.front_matter.title.clone()),
                link: Some(format!("{}/{}", base_url, content.output_name)),
                pub_date: Some(last_modified),
                ..Default::default()
            };
            builder.item(item);
        }
    }
    let rss = builder.build().to_string();

    fs::write(output_path, rss)?;

    Ok(())
}

fn scale_to_height(size: (u32, u32), target_height: u32) -> (u32, u32) {
    let (width, height) = size;

    let scale = (target_height as f32) / (height as f32);
    let target_width = (width as f32) * scale;

    (target_width as u32, target_height)
}

fn generate_thumbnail(content: &Content, output_path: &Utf8Path) -> Result<()> {
    // Add "-thumb" to the output file name.
    let output_file_name = format!(
        "{}-thumb.{}",
        output_path.file_stem().unwrap(),
        output_path.extension().unwrap()
    );
    let output_path = output_path.with_file_name(output_file_name);

    println!("thumbnail {} -> {}", content.source, output_path);

    let src = ImageReader::open(&content.source)?.decode()?;

    let (width, height) = scale_to_height((src.width(), src.height()), 512);
    let thumb = imageops::resize(&src, width, height, FilterType::Lanczos3);
    thumb.save(output_path)?;

    Ok(())
}

fn get_toc_list(
    toc: &mut HashMap<&'static str, Vec<TocItem>>,
    contents: &[Content],
    subdir: &'static str,
) {
    let items = contents
        .iter()
        // Reverse iteration so that newer entries come first.
        .rev()
        .filter_map(|c| {
            let md = if let ContentType::Markdown(md) = &c.content_type {
                md
            } else {
                return None;
            };

            if c.subdir == subdir {
                let title = &md.front_matter.title;
                let date = md.front_matter.date.as_ref().unwrap();
                Some(TocItem {
                    title: title.clone(),
                    date: date.to_string(),
                    target: c.output_name.clone(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    toc.insert(subdir, items);
}

#[derive(Serialize)]
struct TocItem {
    date: String,
    title: String,
    target: String,
}

struct RenderMarkdownState<'a> {
    content: &'a Content,
    markdown_tera_ctx: &'a Context,
    md: &'a MarkdownContent,
    options: &'a ComrakOptions,
    plugins: &'a ComrakPlugins<'a>,
    tera: &'a mut Tera,
    output_path: &'a Utf8Path,
}

fn render_markdown(state: RenderMarkdownState) -> Result<()> {
    let markdown = format!(
        "{}\n{}",
        "{% import \"macros.md\" as macros %}", state.md.body
    );
    state
        .tera
        .add_raw_template(&state.content.output_name, &markdown)?;
    let markdown = state
        .tera
        .render(&state.content.output_name, state.markdown_tera_ctx)?;

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
            .map(|date| date.to_string())
            .unwrap_or_else(|| "?".to_string()),
    );
    let date_format = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second] UTC",
    )?;
    ctx.insert(
        "updated_date",
        &state.content.last_modified.format(&date_format)?,
    );
    ctx.insert("body", &markdown_html);
    let show_home_link = state.content.output_name != "index.html";
    ctx.insert("show_home_link", &show_home_link);
    let html = state.tera.render("base.html", &ctx)?;

    fs::write(state.output_path, html)?;

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
    let adapter = SyntectAdapter::new(Some("base16-ocean.light"));
    let mut render_options = ComrakRenderOptions::default();
    render_options.unsafe_ = true;
    let options = ComrakOptions {
        render: render_options,
        ..Default::default()
    };
    let mut plugins = ComrakPlugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    // Load templates.
    let mut tera = Tera::new("templates/*")?;

    let contents = get_all_contents(&conf)?;

    let mut toc = HashMap::new();
    get_toc_list(&mut toc, &contents, "log");
    get_toc_list(&mut toc, &contents, "notes");

    let mut markdown_tera_ctx = Context::new();
    markdown_tera_ctx.insert("toc", &toc);

    generate_rss(&conf, &contents)?;

    let mut thumbnail_params = Vec::new();

    for content in &contents {
        let output_path = conf.output_dir.join(&content.output_name);

        match &content.content_type {
            ContentType::Markdown(md) => {
                println!("render {} -> {}", content.source, output_path);

                render_markdown(RenderMarkdownState {
                    markdown_tera_ctx: &markdown_tera_ctx,
                    content,
                    md,
                    options: &options,
                    plugins: &plugins,
                    tera: &mut tera,
                    output_path: &output_path,
                })?;
            }
            ContentType::Photo => {
                println!("copy {} -> {}", content.source, output_path);
                fs::copy(&content.source, &output_path)?;

                thumbnail_params.push((content, output_path));
            }
            ContentType::PlainFile => {
                println!("copy {} -> {}", content.source, output_path);
                fs::copy(&content.source, &output_path)?;
            }
        }
    }

    thumbnail_params
        .par_iter()
        .for_each(|(content, output_path)| {
            generate_thumbnail(content, output_path).unwrap()
        });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale() {
        assert_eq!(scale_to_height((1024, 2048), 512), (256, 512));
    }
}
