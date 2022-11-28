//! This example builds an ePub version of TRPL that needs built-in link preprocessor to include
//! sources and to remove invalid HTML comments that causing fatal error found by epubcheck.
//! The book source will download from official github repo and extract to `book/` by default.
//! And then it would build with preprocessor `nocomment` to generate the epub file.
//! Note that it requires you to have crate `mdbook-nocomment` installed.
//!
//! Run this example with:
//!
//! ```
//! cargo run --example build_trpl -- --dest . book/
//! ```

use std::io::{self, Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

// Inject config value to activate the preprocessor `nocomment` with this env var.
// See https://rust-lang.github.io/mdBook/format/configuration/environment-variables.html
const BOOK_PREPROCESSOR: &str = "MDBOOK_PREPROCESSOR__NOCOMMENT";
const BOOK_ARCHIVE_URL: &str = "https://github.com/rust-lang/book/archive/refs/heads/main.zip";
const BOOK_BUF_SIZE: usize = 4 * 1024 * 1024;

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();
    let dest = args.dest.as_path();
    let book_root = args.root.as_path();

    // Download & extract
    let resp = ureq::get(BOOK_ARCHIVE_URL).call()?;
    let mut buf: Vec<u8> = Vec::with_capacity(BOOK_BUF_SIZE);
    resp.into_reader().read_to_end(&mut buf)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(buf))?;
    extract(&mut archive, book_root)?;

    std::env::set_var(BOOK_PREPROCESSOR, "");
    let md = mdbook::MDBook::load(book_root)
        .map_err(|e| anyhow::anyhow!("Could not load mdbook: {}", e))?;
    let outfile = mdbook_epub::output_filename(dest, &md.config);

    match mdbook_epub::generate_with_preprocessor(&md, dest) {
        Ok(_) => writeln!(
            &mut io::stderr(),
            "Successfully wrote epub document to {:?}!",
            outfile,
        )
        .unwrap(),
        Err(err) => writeln!(&mut io::stderr(), "Error: {}", err).unwrap(),
    };
    Ok(())
}

/// Modified `ZipArchive::extract()`. Path::strip_prefix is added to remove top level directory.
fn extract<R: Read + Seek, P: AsRef<Path>>(
    archive: &mut zip::ZipArchive<R>,
    directory: P,
) -> anyhow::Result<()> {
    use std::fs;
    const ZIP_PREFIX: &str = "book-main";

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let filepath = file
            .enclosed_name()
            .ok_or(zip::result::ZipError::InvalidArchive("Invalid file path"))?
            .strip_prefix(ZIP_PREFIX)?;

        let outpath = directory.as_ref().join(filepath);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, StructOpt)]
struct Args {
    #[structopt(
        help = "The book to render.",
        parse(from_os_str),
        default_value = "book/"
    )]
    root: PathBuf,
    #[structopt(short, long, default_value = "target/book/")]
    dest: PathBuf,
}
