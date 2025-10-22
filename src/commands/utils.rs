use std::path::{Path, PathBuf};

use anstream::println;
use hvp_archive::archive::Metadata;
use owo_colors::OwoColorize;

pub fn is_file(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_file() {
        return Err("You need to pass a valid file path.".to_owned());
    }
    Ok(path.to_path_buf())
}

pub fn is_dir(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_dir() {
        return Err("You need to pass a valid dir path.".to_owned());
    }
    Ok(path.to_path_buf())
}

pub fn list_files(input: &Path, without_base: bool) -> Vec<PathBuf> {
    walkdir::WalkDir::new(input)
        .into_iter()
        .filter_map(|e| {
            let f = e.ok()?;
            if f.path().is_dir() {
                return None;
            }
            let path = if without_base {
                f.path().strip_prefix(input).ok()?
            } else {
                f.path()
            };
            Some(path.to_path_buf())
        })
        .collect()
}

/// print the archive metadata to stdout
pub fn print_metadata(metadata: Metadata) {
    println!(
        concat!(
            "{} loaded archive metadata:\n",
            " {dot} game: {:?}\n",
            " {dot} dir count: {}\n",
            " {dot} file count: {}",
        ),
        "[?]".green(),
        metadata.game,
        metadata.dir_count,
        metadata.file_count,
        dot = "|>".cyan(),
    )
}

pub fn progress_bar(len: u64) -> indicatif::ProgressBar {
    indicatif::ProgressBar::new(len)
        .with_style(
            indicatif::ProgressStyle::with_template(
                "{prefix} [{elapsed_precise}] [{bar:40.cyan/blue}] [{pos:>4}/{len:4}] {msg}",
            )
            .unwrap()
            .progress_chars("=> "),
        )
        .with_prefix(
            "[P]"
                .if_supports_color(owo_colors::Stream::Stdout, |t| t.green())
                .to_string(),
        )
}

pub fn prompt() -> anyhow::Result<String> {
    use std::io::BufRead;

    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();

    let mut line = String::new();
    stdin.read_line(&mut line)?;

    Ok(line.trim().to_owned())
}
