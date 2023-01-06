#[macro_use]
extern crate log;

use clap::{value_parser, Parser};
use mdbook::renderer::RenderContext;
use mdbook::MDBook;
use std::io;
use std::path::PathBuf;
use std::process;

use mdbook_epub::Error;

fn main() {
    env_logger::init();
    info!("Booting EPUB generator...");
    let args = Args::parse();
    debug!("generator args = {:?}", args);

    if let Err(e) = run(&args) {
        log::error!("{}", e);

        process::exit(1);
    }
}

fn run(args: &Args) -> Result<(), Error> {
    // get a `RenderContext`, either from stdin (because we're used as a plugin)
    // or by instrumenting MDBook directly (in standalone mode).
    if args.standalone {
        let error = format!(
            "book.toml root file is not found by a path {:?}",
            &args.root.display()
        );
        let md = MDBook::load(&args.root).expect(&error);
        let destination = md.build_dir_for("epub");
        debug!(
            "EPUB book destination folder is : {:?}",
            destination.display()
        );
        debug!("EPUB book config is : {:?}", md.config);
        if args.preprocess {
            mdbook_epub::generate_with_preprocessor(&md, &destination)
        } else {
            let ctx = RenderContext::new(md.root, md.book, md.config, destination);
            mdbook_epub::generate(&ctx)
        }
    } else {
        let ctx: RenderContext =
            serde_json::from_reader(io::stdin()).map_err(|_| Error::RenderContext)?;
        mdbook_epub::generate(&ctx)
    }
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short = 's',
        long = "standalone",
        help = "Run standalone (i.e. not as a mdbook plugin)"
    )]
    standalone: bool,
    #[arg(
        short = 'p',
        long = "preprocess",
        help = "Enable preprocessing for standalone mode."
    )]
    preprocess: bool,
    #[arg(help = "The book to render.", value_parser = value_parser!(PathBuf), default_value = ".")]
    root: PathBuf,
}
