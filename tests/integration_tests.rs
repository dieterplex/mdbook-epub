use epub::doc::EpubDoc;
use log::{debug, error};
use mdbook::renderer::RenderContext;
use mdbook::MDBook;
use mdbook_epub::Error;
use relative_path::RelativePath;
use serial_test::serial;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;
use tempfile::TempDir;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

/// Convenience function for compiling the dummy book into an `EpubDoc`.
fn generate_epub() -> Result<(EpubDoc<BufReader<File>>, PathBuf), Error> {
    let (ctx, _md, temp) = create_dummy_book().unwrap();
    debug!("temp dir = {:?}", &temp);
    mdbook_epub::generate(&ctx)?;
    ensure_epub_opened(temp.path(), &ctx.config)
}

fn generate_epub_preprocessed() -> Result<(EpubDoc<BufReader<File>>, PathBuf), Error> {
    let (_, md, temp) = create_dummy_book().unwrap();
    debug!("temp dir = {:?}", &temp);
    mdbook_epub::generate_with_preprocessor(&md, temp.path())?;
    ensure_epub_opened(temp.path(), &md.config)
}

fn ensure_epub_opened(
    dest: &Path,
    config: &mdbook::Config,
) -> Result<(EpubDoc<BufReader<File>>, PathBuf), Error> {
    let output_file = mdbook_epub::output_filename(dest, config);
    debug!("output_file = {:?}", &output_file.display());

    // let output_file_name = output_file.display().to_string();
    match EpubDoc::new(&output_file) {
        Ok(epub) => {
            let result: (EpubDoc<_>, PathBuf) = (epub, output_file);
            Ok(result)
        }
        Err(err) => {
            error!("dummy book creation error = {}", err);
            Err(Error::EpubDocCreate(output_file.display().to_string()))
        }
    }
}

#[test]
#[serial]
fn output_epub_exists() {
    init_logging();
    let (ctx, _md, temp) = create_dummy_book().unwrap();

    let output_file = mdbook_epub::output_filename(temp.path(), &ctx.config);

    assert!(!output_file.exists());
    mdbook_epub::generate(&ctx).unwrap();
    assert!(output_file.exists());
}

#[test]
#[serial]
fn output_epub_is_valid() {
    init_logging();
    let (ctx, _md, temp) = create_dummy_book().unwrap();
    mdbook_epub::generate(&ctx).unwrap();

    let output_file = mdbook_epub::output_filename(temp.path(), &ctx.config);

    let got = EpubDoc::new(&output_file);

    assert!(got.is_ok());

    // also try to run epubcheck, if it's available
    epub_check(&output_file).unwrap();
}

fn epub_check(path: &Path) -> Result<(), Error> {
    init_logging();
    let cmd = Command::new("epubcheck").arg(path).output();

    match cmd {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                Err(Error::EpubCheck)
            }
        }
        Err(_) => {
            // failed to launch epubcheck, it's probably not installed
            debug!("Failed to launch epubcheck, it's probably not installed here...");
            Err(Error::EpubCheck)
        }
    }
}

#[test]
#[serial]
fn look_for_chapter_1_heading() {
    init_logging();
    debug!("look_for_chapter_1_heading...");
    let mut doc = generate_epub_preprocessed().unwrap();
    debug!("doc current path = {:?}", doc.1);

    let path = if cfg!(target_os = "linux") {
        Path::new("OEBPS").join("chapter_1.html") // linux
    } else {
        Path::new("OEBPS/chapter_1.html").to_path_buf() // windows with 'forward slash' /
    };
    debug!("short path = {:?}", path.display().to_string());
    debug!("full path = {:?}", &doc.1);
    let file = doc.0.get_resource_str_by_path(path);
    debug!("file = {:?}", &file);
    let content = file.unwrap();
    debug!("content = {:?}", content.len());
    assert!(content.contains("<h1>Chapter 1</h1>"));
    assert!(!content.contains("{{#rustdoc_include"));
    assert!(content.contains("fn main() {"));
}

#[test]
#[serial]
fn rendered_document_contains_all_chapter_files_and_assets() {
    init_logging();
    debug!("rendered_document_contains_all_chapter_files_and_assets...");
    let chapters = vec!["chapter_1.html", "rust-logo.png"];
    let mut doc = generate_epub().unwrap();
    debug!(
        "doc current path = {:?} / {:?}",
        doc.0.get_current_path(),
        doc.1
    );

    for chapter in chapters {
        let path = if cfg!(target_os = "windows") {
            Path::new("OEBPS/").join(chapter) // windows with 'forward slash' /
        } else {
            Path::new("OEBPS").join(chapter) // linux
        };
        // let path = path.display().to_string();
        debug!("path = {}", &path.display().to_string());
        let got = doc.0.get_resource_by_path(&path);
        debug!("got = {:?}", got.is_ok());
        assert!(got.is_ok(), "{}", &path.display().to_string());
    }
}

#[test]
#[serial]
fn render_only_draft_chapters_containing_sub() {
    init_logging();
    let (mut doc, _) = generate_epub().unwrap();
    let items = vec![
        ("Draft_sub.html", "Draft_sub"),
        ("draft_1.html", "draft_1"),
        ("draft/draft_2.html", "draft_2"),
    ];
    let oebps = RelativePath::new("OEBPS");
    for it in items {
        let path = oebps.join(it.0);
        let got = doc.get_resource_by_path(path.as_str()).unwrap();
        let content = String::from_utf8_lossy(got.as_ref());
        let pat = format!("<title>{name}</title>", name = it.1);
        assert!(&content.contains(&pat))
    }
    let path = oebps.join("Draft_simple.html");
    let got = doc.get_resource_by_path(path.as_str());
    assert!(got.is_err(), "should not embed draft chapter");
}

#[test]
#[serial]
fn straight_quotes_transformed_into_curly_quotes() {
    init_logging();
    debug!("straight_quotes_transformed_into_curly_quotes...");
    let mut doc = generate_epub().unwrap();
    debug!("doc current path = {:?}", doc.1);

    let path = if cfg!(target_os = "linux") {
        Path::new("OEBPS").join("chapter_1.html") // linux
    } else {
        Path::new("OEBPS/chapter_1.html").to_path_buf() // windows with 'forward slash' /
    };
    let file = doc.0.get_resource_str_by_path(path);
    let content = file.unwrap();
    debug!("content = {:?}", content);
    assert!(content.contains("<p>“One morning, when Gregor Samsa woke from troubled dreams, he found himself ‘transformed’ in his bed into a horrible vermin.”</p>"));
}

/// Use `MDBook::load()` to load the dummy book into memory, then set up the
/// `RenderContext` for use the EPUB generator.
fn create_dummy_book() -> Result<(RenderContext, MDBook, TempDir), Error> {
    let temp = TempDir::with_prefix("mdbook-epub")?;

    let dummy_book = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("dummy");
    debug!("dummy_book = {:?}", &dummy_book.display().to_string());

    let md = MDBook::load(dummy_book);

    let book = md.expect("dummy MDBook is not loaded");
    let ctx = RenderContext::new(
        book.root.clone(),
        book.book.clone(),
        book.config.clone(),
        temp.path().to_path_buf(),
    );

    Ok((ctx, book, temp))
}
