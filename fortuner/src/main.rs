use anyhow::{Result, anyhow, bail};
use clap::Parser;
use rand::prelude::SliceRandom;
use rand::{RngCore, SeedableRng, rngs::StdRng};
use regex::RegexBuilder;
use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::PathBuf,
};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(author, version, about)]
/// Rust version of `fortune`
struct Args {
    /// Input files or directories
    #[arg(required(true), value_name = "FILE")]
    sources: Vec<String>,

    /// Pattern
    #[arg(short('m'), long)]
    pattern: Option<String>,

    /// Case-insensitive pattern matching
    #[arg(short, long)]
    insensitive: bool,

    /// Random seed
    #[arg(short, long, value_parser(clap::value_parser!(u64)))]
    seed: Option<u64>,
}

#[derive(Debug)]
struct Fortune {
    source: String,
    text: String,
}

fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    let pattern = args
        .pattern
        .map(|val: String| {
            RegexBuilder::new(val.as_str())
                .case_insensitive(args.insensitive)
                .build()
                .map_err(|_| anyhow!(r#"Invalid --pattern "{val}""#))
        })
        .transpose()?;

    let files = find_files(&args.sources)?;
    let fortunes = read_fortunes(&files)?;

    match pattern {
        Some(pattern) => {
            let mut prev_source = None;
            for fortune in fortunes.iter().filter(|f| pattern.is_match(&f.text)) {
                if prev_source.as_ref().map_or(true, |s| s != &fortune.source) {
                    eprintln!("({})\n%", fortune.source);
                    prev_source = Some(fortune.source.clone());
                }
                println!("{}\n%", fortune.text);
            }
        }
        _ => {
            println!(
                "{}",
                pick_fortune(&fortunes, args.seed)
                    .or_else(|| Some("No fortunes found".to_string()))
                    .unwrap()
            );
        }
    }

    Ok(())
}

fn find_files(paths: &[String]) -> Result<Vec<PathBuf>> {
    let dat = OsStr::new("dat");
    let mut files = vec![];

    for path in paths {
        match fs::metadata(path) {
            Err(e) => bail!("{path}: {e}"),
            Ok(_) => files.extend(
                WalkDir::new(path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|f| f.file_type().is_file() && f.path().extension() != Some(dat))
                    .map(|f| f.path().into()),
            ),
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn read_fortunes(paths: &[PathBuf]) -> Result<Vec<Fortune>> {
    let mut fortunes = vec![];
    let mut buffer = vec![];

    for path in paths {
        let basename = path.file_name().unwrap().to_string_lossy().into_owned();
        let file = File::open(path).map_err(|e| anyhow!("{}: {e}", path.to_string_lossy()))?;

        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if line == "%" {
                if !buffer.is_empty() {
                    fortunes.push(Fortune {
                        source: basename.clone(),
                        text: buffer.join("\n"),
                    });
                    buffer.clear();
                }
            } else {
                buffer.push(line);
            }
        }
    }

    Ok(fortunes)
}

fn pick_fortune(fortunes: &[Fortune], seed: Option<u64>) -> Option<String> {
    let mut rng: Box<dyn RngCore> = match seed {
        Some(val) => Box::new(StdRng::seed_from_u64(val)),
        _ => Box::new(rand::thread_rng()),
    };

    fortunes.choose(&mut rng).map(|f| f.text.to_string())
}
