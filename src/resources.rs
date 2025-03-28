use html_parser::{Dom, Node};
use mdbook::book::BookItem;
use mdbook::renderer::RenderContext;
use mdbook::utils::new_cmark_parser;
use mime_guess::Mime;
use pulldown_cmark::{Event, Tag};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::Wrapping;
use std::path::{Component, Path, PathBuf};
use url::Url;

use crate::Error;

pub(crate) fn find(ctx: &RenderContext) -> Result<HashMap<String, Asset>, Error> {
    let mut assets: HashMap<String, Asset> = HashMap::new();
    debug!("Finding resources by:\n{:?}", ctx.config);
    let src_dir = ctx.root.join(&ctx.config.book.src).canonicalize()?;

    debug!(
        "Start iteration over a [{:?}] sections in src_dir = {:?}",
        ctx.book.sections.len(),
        src_dir
    );
    for section in ctx.book.iter() {
        match *section {
            BookItem::Chapter(ref ch) => {
                debug!("Searching links and assets for: {}", ch);
                if ch.path.is_none() {
                    debug!("{} is a draft chapter and should be no content.", ch.name);
                    continue;
                }
                for link in find_assets_in_markdown(&ch.content)? {
                    let asset = match Url::parse(&link) {
                        Ok(url) => Asset::from_url(url, &ctx.destination),
                        Err(_) => Asset::from_local(&link, &src_dir, ch.path.as_ref().unwrap()),
                    }?;
                    assets.insert(link, asset);
                }
            }
            BookItem::Separator => trace!("Skip separator."),
            BookItem::PartTitle(ref title) => trace!("Skip part title: {}.", title),
        }
    }

    Ok(assets)
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub(crate) enum AssetKind {
    Remote(Url),
    Local(PathBuf),
}

#[derive(Clone, Debug)]
pub(crate) struct Asset {
    /// The asset's absolute location on disk.
    pub(crate) location_on_disk: PathBuf,
    /// The asset's filename relative to the `src/` directory. If it's a remote
    /// asset it relative to the destination where the book generated.
    pub(crate) filename: PathBuf,
    pub(crate) mimetype: Mime,
    /// The asset's original link as a enum [local][AssetKind::Local] or [remote][AssetKind::Remote].
    pub(crate) source: AssetKind,
}

impl PartialEq for Asset {
    fn eq(&self, other: &Self) -> bool {
        self.location_on_disk == other.location_on_disk && self.mimetype == other.mimetype
    }
}

impl Eq for Asset {}

impl Hash for Asset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Use Wrapping<u64> to allow wrapping overflow
        let mut sum = Wrapping::default();

        let mut hasher = DefaultHasher::new();
        Hash::hash(&self.location_on_disk, &mut hasher);
        sum += hasher.finish();

        let mut hasher = DefaultHasher::new();
        Hash::hash(&self.mimetype, &mut hasher);
        sum += hasher.finish();

        state.write_u64(sum.0);
    }
}
impl Asset {
    pub(crate) fn new<P, Q, K>(filename: P, absolute_location: Q, source: K) -> Self
    where
        P: Into<PathBuf>,
        Q: Into<PathBuf>,
        K: Into<AssetKind>,
    {
        let location_on_disk = absolute_location.into();
        let mt = mime_guess::from_path(&location_on_disk).first_or_octet_stream();
        let source = source.into();
        Self {
            location_on_disk,
            filename: filename.into(),
            mimetype: mt,
            source,
        }
    }

    fn from_url(url: Url, dest_dir: &Path) -> Result<Asset, Error> {
        let filename = hash_link(&url);
        let dest_dir = normalize_path(dest_dir);
        let full_filename = dest_dir.join("cache").join(filename);
        // Will fetch assets to normalized path later. fs::canonicalize() only works for existed path.
        let absolute_location = normalize_path(full_filename.as_path());
        let filename = absolute_location.strip_prefix(dest_dir).unwrap();
        let asset = Asset::new(filename, &absolute_location, AssetKind::Remote(url));
        trace!("{:#?}", asset);
        Ok(asset)
    }

    fn from_local(link: &str, src_dir: &Path, chapter_path: &Path) -> Result<Asset, Error> {
        let full_path = src_dir.join(chapter_path);
        let relative_link = PathBuf::from(link);
        // Since chapter_path is some file and joined with src_dir, it's safe to
        // unwrap parent here.
        let full_filename = full_path.parent().unwrap().join(&relative_link);
        let absolute_location = full_filename
            .canonicalize()
            .map_err(|_| Error::AssetFileNotFound(format!("Asset was not found: {link}")))?;
        if !absolute_location.is_file() {
            return Err(Error::AssetFile(absolute_location));
        }
        // Use filename as embedded file path with content from absolute_location.
        let filename = if full_filename.is_symlink() {
            debug!(
                "Strip symlinked asset '{:?}' prefix without canonicalized path.",
                &relative_link
            );
            full_filename.strip_prefix(src_dir).unwrap()
        } else {
            absolute_location.strip_prefix(src_dir).unwrap()
        };
        let asset = Asset::new(
            filename,
            &absolute_location,
            AssetKind::Local(relative_link),
        );
        trace!("{:#?}", asset);
        Ok(asset)
    }
}

// Look up resources in chapter md content
fn find_assets_in_markdown(chapter_src_content: &str) -> Result<Vec<String>, Error> {
    let mut found_asset = Vec::new();

    // Look up resources in nested HTML element
    fn find_assets_in_nested_html_tags(element: &html_parser::Element, found: &mut Vec<String>) {
        if element.name == "img" {
            if let Some(dest) = &element.attributes["src"] {
                found.push(dest.clone());
            }
        }
        for item in &element.children {
            if let Node::Element(ref nested_element) = item {
                find_assets_in_nested_html_tags(nested_element, found);
            }
        }
    }

    for event in new_cmark_parser(chapter_src_content, false) {
        match event {
            Event::Start(Tag::Image {
                link_type: _,
                dest_url,
                title: _,
                id: _,
            }) => {
                found_asset.push(dest_url.to_string());
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                let content = html.into_string();
                if let Ok(dom) = Dom::parse(&content) {
                    for item in dom.children {
                        if let Node::Element(ref element) = item {
                            find_assets_in_nested_html_tags(element, &mut found_asset)
                        }
                    }
                }
            }
            _ => {}
        }
    }

    found_asset.sort();
    found_asset.dedup();
    if !found_asset.is_empty() {
        trace!("Assets found in content : {:?}", found_asset);
    }
    Ok(found_asset)
}

pub(crate) fn hash_link(url: &Url) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let path = PathBuf::from(url.path());
    let ext = path.extension().and_then(OsStr::to_str).unwrap_or_else(|| {
        debug!("Unable to extract file ext from {}", url);
        ""
    });
    if ext.is_empty() {
        format!("{:x}", hasher.finish())
    } else {
        format!("{:x}.{}", hasher.finish(), ext)
    }
}

// From cargo/util/paths.rs
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub(crate) mod handler {
    use std::{
        fs::{self, File, OpenOptions},
        io::{self, Read},
        path::Path,
    };

    #[cfg(test)]
    use mockall::automock;

    use crate::Error;

    use super::{Asset, AssetKind};

    #[cfg_attr(test, automock)]
    pub(crate) trait ContentRetriever {
        fn download(&self, asset: &Asset) -> Result<(), Error> {
            if let AssetKind::Remote(url) = &asset.source {
                let dest = &asset.location_on_disk;
                if dest.is_file() {
                    debug!("Cache file {:?} to {} already exists.", dest, url);
                } else {
                    if let Some(cache_dir) = dest.parent() {
                        fs::create_dir_all(cache_dir)?;
                    }
                    debug!("Downloading asset : {}", url);
                    let mut file = OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .write(true)
                        .open(dest)?;
                    let mut resp = self.retrieve(url.as_str())?;
                    io::copy(&mut resp, &mut file)?;
                }
            }
            Ok(())
        }
        fn read(&self, path: &Path, buffer: &mut Vec<u8>) -> Result<(), Error> {
            File::open(path)?.read_to_end(buffer)?;
            Ok(())
        }
        fn retrieve(&self, url: &str) -> Result<Box<(dyn Read + Send + Sync + 'static)>, Error>;
    }

    pub(crate) struct ResourceHandler;
    impl ContentRetriever for ResourceHandler {
        fn retrieve(&self, url: &str) -> Result<Box<(dyn Read + Send + Sync + 'static)>, Error> {
            let res = ureq::get(url).call()?;
            match res.status() {
                200 => Ok(res.into_reader()),
                404 => Err(Error::AssetFileNotFound(format!(
                    "Missing remote resource: {url}"
                ))),
                _ => unreachable!("Unexpected response status"),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::ContentRetriever;
        use crate::{resources::Asset, Error};
        use tempfile::TempDir;

        type BoxRead = Box<(dyn std::io::Read + Send + Sync + 'static)>;

        #[test]
        fn download_success() {
            use std::io::Read;

            struct TestHandler;
            impl ContentRetriever for TestHandler {
                fn retrieve(&self, _url: &str) -> Result<BoxRead, Error> {
                    Ok(Box::new("donwload content".as_bytes()))
                }
            }
            let cr = TestHandler {};
            let a = temp_remote_asset("https://mdbook-epub.org/image.svg").unwrap();
            let r = cr.download(&a);

            assert!(r.is_ok());
            let mut buffer = String::new();
            let mut f = std::fs::File::open(&a.location_on_disk).unwrap();
            f.read_to_string(&mut buffer).unwrap();
            assert_eq!(buffer, "donwload content");
        }

        #[test]
        fn download_fail_when_resource_not_exist() {
            struct TestHandler;
            impl ContentRetriever for TestHandler {
                fn retrieve(&self, url: &str) -> Result<BoxRead, Error> {
                    Err(Error::AssetFileNotFound(format!(
                        "Missing remote resource: {url}",
                    )))
                }
            }
            let cr = TestHandler {};
            let a = temp_remote_asset("https://mdbook-epub.org/not-exist.svg").unwrap();
            let r = cr.download(&a);

            assert!(r.is_err());
            assert!(matches!(r.unwrap_err(), Error::AssetFileNotFound(_)));
        }

        #[test]
        #[should_panic(expected = "NOT 200 or 404")]
        fn download_fail_with_unexpected_status() {
            struct TestHandler;
            impl ContentRetriever for TestHandler {
                fn retrieve(&self, _url: &str) -> Result<BoxRead, Error> {
                    panic!("NOT 200 or 404")
                }
            }
            let cr = TestHandler {};
            let a = temp_remote_asset("https://mdbook-epub.org/bad.svg").unwrap();
            let r = cr.download(&a);

            panic!("{}", r.unwrap_err().to_string());
        }

        fn temp_remote_asset(url: &str) -> Result<Asset, Error> {
            let dest_dir = TempDir::with_prefix("mdbook-epub")?;
            Asset::from_url(url::Url::parse(url).unwrap(), dest_dir.path())
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn find_images() {
        let parent_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/dummy/src");
        let src =
            "![Image 1](./rust-logo.png)\n[a link](to/nowhere) ![Image 2][2]\n\n[2]: reddit.svg\n\
            \n\n<img alt=\"Rust Logo in html\" src=\"rust-logo.svg\" class=\"center\" style=\"width: 20%;\" />\n\n\
            ![Image 4](./rust-logo.png)\n[a link](to/nowhere)";
        let should_be = vec![
            parent_dir.join("rust-logo.png").canonicalize().unwrap(),
            parent_dir.join("reddit.svg").canonicalize().unwrap(),
            parent_dir.join("rust-logo.svg").canonicalize().unwrap(),
        ];

        let got = find_assets_in_markdown(src)
            .unwrap()
            .into_iter()
            .map(|a| parent_dir.join(a).canonicalize().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(got, should_be);
    }

    #[test]
    fn find_local_asset() {
        let link = "./rust-logo.png";
        let link2 = "./epub-logo.svg";
        let temp = tempfile::TempDir::with_prefix("mdbook-epub").unwrap();
        let dest_dir = temp.path().to_string_lossy().to_string();
        let chapters = json!([
        {"Chapter": {
            "name": "Chapter 1",
            "content": format!("# Chapter 1\r\n\r\n![Image]({link})\r\n![Image]({link2})"),
            "number": [1],
            "sub_items": [],
            "path": "chapter_1.md",
            "parent_names": []}}]);
        let ctx = ctx_with_chapters(&chapters, &dest_dir).unwrap();

        let mut assets = find(&ctx).unwrap();
        assert!(assets.len() == 2);

        fn assert_asset(a: Asset, link: &str, ctx: &RenderContext) {
            let path = PathBuf::from(link);
            let filename = normalize_path(&path);
            let absolute_location = PathBuf::from(&ctx.root)
                .join(&ctx.config.book.src)
                .join(&filename)
                .canonicalize()
                .unwrap();
            let source = AssetKind::Local(path);
            let should_be = Asset::new(filename, absolute_location, source);
            assert_eq!(a, should_be);
        }
        assert_asset(assets.remove(link).unwrap(), link, &ctx);
        assert_asset(assets.remove(link2).unwrap(), link2, &ctx);
    }

    #[test]
    fn find_remote_asset() {
        let link = "https://www.rust-lang.org/static/images/rust-logo-blk.svg";
        let link2 = "https://www.rust-lang.org/static/images/rust-logo-blk.png";
        let link_parsed = Url::parse(link).unwrap();
        let temp = tempfile::TempDir::with_prefix("mdbook-epub").unwrap();
        let dest_dir = temp.path().to_string_lossy().to_string();
        let chapters = json!([
        {"Chapter": {
            "name": "Chapter 1",
            "content": format!("# Chapter 1\r\n\r\n![Image]({link})\r\n<a href=\"\"><img  src=\"{link2}\"></a>"),
            "number": [1],
            "sub_items": [],
            "path": "chapter_1.md",
            "parent_names": []}}]);
        let ctx = ctx_with_chapters(&chapters, &dest_dir).unwrap();

        let mut assets = find(&ctx).unwrap();

        assert!(assets.len() == 2);
        let got = assets.remove(link).unwrap();

        let filename = PathBuf::from("cache").join(hash_link(&link_parsed));
        let absolute_location = temp.path().join(&filename);
        let source = AssetKind::Remote(link_parsed);
        let should_be = Asset::new(filename, absolute_location, source);
        assert_eq!(got, should_be);
    }

    #[test]
    fn find_draft_chapter_without_error() {
        let temp = tempfile::TempDir::with_prefix("mdbook-epub").unwrap();
        let dest_dir = temp.into_path().to_string_lossy().to_string();
        let chapters = json!([
        {"Chapter": {
            "name": "Chapter 1",
            "content": "",
            "number": [1],
            "sub_items": [],
            "path": null,
            "parent_names": []}}]);
        let ctx = ctx_with_chapters(&chapters, &dest_dir).unwrap();
        assert!(find(&ctx).unwrap().is_empty());
    }

    #[test]
    #[should_panic(expected = "Asset was not found")]
    fn find_asset_fail_when_chapter_dir_not_exist() {
        panic!(
            "{}",
            Asset::from_local("a.png", Path::new("tests/dummy/src"), Path::new("ch/a.md"))
                .unwrap_err()
                .to_string()
        );
    }

    #[test]
    #[should_panic(expected = "Asset was not a file")]
    fn find_asset_fail_when_it_is_a_dir() {
        panic!(
            "{}",
            Asset::from_local(
                "wikimedia",
                Path::new("tests/dummy"),
                Path::new("third_party/a.md")
            )
            .unwrap_err()
            .to_string()
        );
    }

    fn ctx_with_chapters(
        chapters: &Value,
        destination: &str,
    ) -> Result<RenderContext, mdbook::errors::Error> {
        let json_ctx = json!({
            "version": mdbook::MDBOOK_VERSION,
            "root": "tests/dummy",
            "book": {"sections": chapters, "__non_exhaustive": null},
            "config": {
                "book": {"authors": [], "language": "en", "multilingual": false,
                    "src": "src", "title": "DummyBook"},
                "output": {"epub": {"curly-quotes": true}}},
            "destination": destination
        });
        RenderContext::from_json(json_ctx.to_string().as_bytes())
    }
}
