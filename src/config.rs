use epub_builder::EpubVersion;
use mdbook::renderer::RenderContext;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::Error;

pub const DEFAULT_TEMPLATE: &str = include_str!("index.hbs");

/// The configuration struct used to tweak how an EPUB document is generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    /// A list of additional stylesheets to include in the document.
    pub additional_css: Vec<PathBuf>,
    /// Should we use the default stylesheet (default: true)?
    pub use_default_css: bool,
    /// The template file to use when rendering individual chapters (relative
    /// to the book root).
    pub index_template: Option<PathBuf>,
    /// A cover image to use for the epub.
    pub cover_image: Option<PathBuf>,
    /// Additional assets to include in the ebook, such as typefaces.
    pub additional_resources: Vec<PathBuf>,
    /// Don't render section labels.
    pub no_section_label: bool,
    /// Use "smart quotes" instead of the usual `"` character.
    pub curly_quotes: bool,
    /// EPUB version to use if specified, otherwise defaults to the epub-builder default.
    epub_version: Option<u8>,
}

impl Config {
    /// Get the `output.epub` table from the provided `book.toml` config,
    /// falling back to the default if
    pub fn from_render_context(ctx: &RenderContext) -> Result<Config, Error> {
        match ctx.config.get("output.epub") {
            Some(table) => {
                let mut cfg: Config = table.clone().try_into()?;

                // make sure we update the `index_template` to make it relative
                // to the book root
                if let Some(template_file) = cfg.index_template.take() {
                    cfg.index_template = Some(ctx.root.join(template_file));
                }

                Ok(cfg)
            }
            None => Ok(Config::default()),
        }
    }

    pub fn template(&self) -> Result<String, Error> {
        match self.index_template {
            Some(ref filename) => {
                let buffer = std::fs::read_to_string(filename)
                    .map_err(|_| Error::OpenTemplate(filename.clone()))?;

                Ok(buffer)
            }
            None => Ok(DEFAULT_TEMPLATE.to_string()),
        }
    }

    pub fn epub_version(&self) -> Result<EpubVersion, Error> {
        match self.epub_version {
            Some(2) | None => Ok(EpubVersion::V20),
            Some(3) => Ok(EpubVersion::V30),
            Some(v) => Err(Error::EpubDocCreate(format!(
                "Unsupported epub version specified in book.toml: {}",
                v
            ))),
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            use_default_css: true,
            additional_css: Vec::new(),
            index_template: None,
            cover_image: None,
            additional_resources: Vec::new(),
            no_section_label: false,
            curly_quotes: false,
            epub_version: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_epub_version() {
        let c = Config::from_render_context(&ctx_with_version(None)).unwrap();
        assert_eq!(EpubVersion::V20, c.epub_version().unwrap());

        let c = Config::from_render_context(&ctx_with_version(Some(2))).unwrap();
        assert_eq!(EpubVersion::V20, c.epub_version().unwrap());

        let c = Config::from_render_context(&ctx_with_version(Some(3))).unwrap();
        assert_eq!(EpubVersion::V30, c.epub_version().unwrap());

        let c = Config::from_render_context(&ctx_with_version(Some(42))).unwrap();
        assert!(matches!(c.epub_version(), Err(Error::EpubDocCreate(_))));
    }

    fn ctx_with_version(ver: Option<u8>) -> RenderContext {
        let options = match ver {
            Some(v) => json!({"epub-version": v}),
            None => json!({}),
        };
        let ctx = json!({
            "version": mdbook::MDBOOK_VERSION,
            "root": "tests/dummy",
            "book": {"sections": [{
                "Chapter": {
                    "name": "Chapter 1",
                    "content": "",
                    "number": [1],
                    "sub_items": [],
                    "path": "chapter_1.md",
                    "parent_names": []
                }}], "__non_exhaustive": null},
            "config": {
                "book": {"authors": [], "language": "en", "multilingual": false,
                    "src": "src", "title": "DummyBook"},
                "output": {"epub": options}},
            "destination": "."
        });
        RenderContext::from_json(ctx.to_string().as_bytes()).unwrap()
    }
}
