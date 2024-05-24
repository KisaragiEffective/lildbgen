use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;
use clap::Parser;
use url::Url;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    display_name: String,
    #[clap(long)]
    url: Option<Url>,
    #[clap(long)]
    installed_base_directory: PathBuf,
    #[clap(long)]
    output_file: PathBuf,
    #[clap(long)]
    non_sorted: bool,
}

fn main() {
    let args = Args::parse();
    if args.non_sorted {
        let mut collection = Vec::new();
        let inst = Instant::now();
        gather_guid(&args.installed_base_directory, &mut collection);
        eprintln!("jobs.gather: {:?}", inst.elapsed());
        print_all(&args.display_name, args.url, &args.output_file, collection);
        eprintln!("jobs.print: {:?}", inst.elapsed());
    } else {
        let mut collection = BTreeSet::new();
        let inst = Instant::now();
        gather_guid(&args.installed_base_directory, &mut collection);
        eprintln!("jobs.gather: {:?}", inst.elapsed());
        print_all(&args.display_name, args.url, &args.output_file, collection);
        eprintln!("jobs.print: {:?}", inst.elapsed());
    }
}

trait Insert<T> {
    fn insert(&mut self, v: T);
}

impl<T: Ord> Insert<T> for BTreeSet<T> {
    fn insert(&mut self, v: T) {
        self.insert(v);
    }
}

impl<T> Insert<T> for Vec<T> {
    fn insert(&mut self, v: T) {
        self.push(v);
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Clone)]
struct GUID(Box<str>);

impl FromStr for GUID {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = s.len() != 32 || s.bytes().any(|x| !matches!(x, b'0'..=b'9' | b'a'..=b'f'));
        if invalid {
            return Err(())
        }

        Ok(Self(s.to_string().into_boxed_str()))
    }
}

fn gather_guid<C: Insert<GUID>>(base_dir: &Path, col: &mut C) {
    fn gather_guid_helper<'a, R: 'a + Insert<GUID>>(base_dir: &Path, col: &'a mut R) -> &'a mut R {
        let mut r = col;
        for x in std::fs::read_dir(base_dir).unwrap() {
            let x = x.unwrap();
            let child_path = x.path();
            if x.file_type().unwrap().is_dir() {
                r = gather_guid_helper(&child_path, r);
            } else if child_path.extension() == Some(OsStr::new("meta")) {
                // eprintln!("check: {child_path}", child_path = child_path.display());
                let g = std::fs::read_to_string(child_path).unwrap()
                    .lines()
                    .find(|x| x.starts_with("guid:"))
                    .and_then(|x| x.split_once(':'))
                    .map(|x| x.1)
                    .map(|x| x.strip_prefix(' ').unwrap_or(x))
                    .map(|x| x.strip_suffix('\r').unwrap_or(x))
                    .unwrap()
                    .parse::<GUID>()
                    .unwrap();

                r.insert(g);
            }
        }

        r
    }

    gather_guid_helper(base_dir, col);
}

fn print_all(display_name: &str, url: Option<Url>, out: &Path, col: impl IntoIterator<Item = GUID>) {
    let mut bw = BufWriter::new(File::options().write(true).append(false).open(out).expect("failed to open file"));

    bw.write(b"displayName: ").unwrap();
    bw.write(display_name.as_bytes()).unwrap();
    bw.write(b"\n").unwrap();

    if let Some(url) = url {
        bw.write(b"url: ").unwrap();
        bw.write(url.as_str().as_bytes()).unwrap();
        bw.write(b"\n").unwrap();
    }

    bw.write(b"guids:\n").unwrap();
    for c in col {
        bw.write(c.0.as_bytes()).unwrap();
        bw.write(b"\n").unwrap();
    }

    bw.flush().unwrap();
}
