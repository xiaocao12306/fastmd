use std::fmt;
use std::path::{Path, PathBuf};

/// Source used to produce a hovered Nautilus candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverCandidateSource {
    /// A direct file-system path carried by an AT-SPI path-like attribute.
    AtspiPathAttribute,
    /// A `file://`-style URI attribute normalized into a file-system path.
    AtspiUriAttribute,
    /// A visible hovered-row label reconstructed against the front directory.
    HoveredRowLabelWithParentDirectory,
    /// Test-only fixture input.
    ValidationFixture,
}

impl HoverCandidateSource {
    /// Stable human-readable diagnostic label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::AtspiPathAttribute => "atspi-path-attribute",
            Self::AtspiUriAttribute => "atspi-uri-attribute",
            Self::HoveredRowLabelWithParentDirectory => "hovered-row-label+parent-directory",
            Self::ValidationFixture => "validation-fixture",
        }
    }
}

/// Candidate item surfaced by Nautilus-specific integration code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoverCandidate {
    LocalPath {
        path: PathBuf,
        source: HoverCandidateSource,
    },
    UnsupportedItem {
        description: String,
        source: HoverCandidateSource,
    },
}

/// A hovered item that already satisfies the current macOS acceptance rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedMarkdownPath {
    path: PathBuf,
    source: HoverCandidateSource,
}

impl AcceptedMarkdownPath {
    pub fn new(path: PathBuf, source: HoverCandidateSource) -> Self {
        Self { path, source }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn source(&self) -> HoverCandidateSource {
        self.source
    }
}

/// Reasons a hovered Nautilus candidate does not satisfy the macOS-matching
/// file acceptance rules yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoverCandidateRejection {
    UnsupportedItem {
        description: String,
        source: HoverCandidateSource,
    },
    RelativePath {
        path: PathBuf,
        source: HoverCandidateSource,
    },
    MissingPath {
        path: PathBuf,
        source: HoverCandidateSource,
    },
    Directory {
        path: PathBuf,
        source: HoverCandidateSource,
    },
    UnsupportedExtension {
        path: PathBuf,
        extension: Option<String>,
        source: HoverCandidateSource,
    },
}

impl fmt::Display for HoverCandidateRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedItem {
                description,
                source,
            } => write!(
                f,
                "unsupported hovered item from {:?}: {}",
                source, description
            ),
            Self::RelativePath { path, source } => write!(
                f,
                "relative hovered path rejected from {:?}: {}",
                source,
                path.display()
            ),
            Self::MissingPath { path, source } => write!(
                f,
                "missing hovered path from {:?}: {}",
                source,
                path.display()
            ),
            Self::Directory { path, source } => write!(
                f,
                "hovered directory rejected from {:?}: {}",
                source,
                path.display()
            ),
            Self::UnsupportedExtension {
                path,
                extension,
                source,
            } => write!(
                f,
                "hovered path with unsupported extension from {:?}: {} ({})",
                source,
                path.display(),
                extension.as_deref().unwrap_or("<none>")
            ),
        }
    }
}

impl std::error::Error for HoverCandidateRejection {}

/// Mirrors the current macOS `FinderItemResolver` path acceptance rule:
/// existing local `.md` files only.
#[derive(Clone, Debug, Default)]
pub struct LinuxMarkdownFilter;

impl LinuxMarkdownFilter {
    pub const MARKDOWN_EXTENSION: &'static str = "md";

    pub fn accept_candidate(
        &self,
        candidate: HoverCandidate,
    ) -> Result<AcceptedMarkdownPath, HoverCandidateRejection> {
        match candidate {
            HoverCandidate::LocalPath { path, source } => self.accept_local_path(path, source),
            HoverCandidate::UnsupportedItem {
                description,
                source,
            } => Err(HoverCandidateRejection::UnsupportedItem {
                description,
                source,
            }),
        }
    }

    fn accept_local_path(
        &self,
        path: PathBuf,
        source: HoverCandidateSource,
    ) -> Result<AcceptedMarkdownPath, HoverCandidateRejection> {
        if !path.is_absolute() {
            return Err(HoverCandidateRejection::RelativePath { path, source });
        }

        if !path.exists() {
            return Err(HoverCandidateRejection::MissingPath { path, source });
        }

        if path.is_dir() {
            return Err(HoverCandidateRejection::Directory { path, source });
        }

        let extension = path.extension().and_then(|value| value.to_str());
        let is_markdown = extension
            .map(|value| value.eq_ignore_ascii_case(Self::MARKDOWN_EXTENSION))
            .unwrap_or(false);

        if !is_markdown {
            return Err(HoverCandidateRejection::UnsupportedExtension {
                extension: extension.map(ToOwned::to_owned),
                path,
                source,
            });
        }

        Ok(AcceptedMarkdownPath::new(path, source))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HoverCandidate, HoverCandidateRejection, HoverCandidateSource, LinuxMarkdownFilter,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug)]
    struct TempFixture {
        root: PathBuf,
    }

    impl TempFixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "fastmd-platform-linux-nautilus-{nonce}-{}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("temp directory should be created");
            Self { root }
        }

        fn write_file(&self, relative_path: impl AsRef<Path>, contents: &str) -> PathBuf {
            let path = self.root.join(relative_path);
            fs::write(&path, contents).expect("temp file should be written");
            path
        }

        fn create_directory(&self, relative_path: impl AsRef<Path>) -> PathBuf {
            let path = self.root.join(relative_path);
            fs::create_dir_all(&path).expect("temp directory should be created");
            path
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn accepts_existing_markdown_files_case_insensitively() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.MD", "# hi");
        let filter = LinuxMarkdownFilter;

        let accepted = filter
            .accept_candidate(HoverCandidate::LocalPath {
                path: path.clone(),
                source: HoverCandidateSource::ValidationFixture,
            })
            .expect("markdown file should be accepted");

        assert_eq!(accepted.path(), path.as_path());
        assert_eq!(accepted.source(), HoverCandidateSource::ValidationFixture);
    }

    #[test]
    fn rejects_directories_even_if_the_name_looks_like_markdown() {
        let fixture = TempFixture::new();
        let path = fixture.create_directory("folder.md");
        let filter = LinuxMarkdownFilter;

        let rejection = filter
            .accept_candidate(HoverCandidate::LocalPath {
                path: path.clone(),
                source: HoverCandidateSource::ValidationFixture,
            })
            .expect_err("directory should be rejected");

        assert_eq!(
            rejection,
            HoverCandidateRejection::Directory {
                path,
                source: HoverCandidateSource::ValidationFixture,
            }
        );
    }

    #[test]
    fn rejects_missing_relative_and_unsupported_items() {
        let fixture = TempFixture::new();
        let txt = fixture.write_file("notes.txt", "hi");
        let filter = LinuxMarkdownFilter;

        let missing = filter
            .accept_candidate(HoverCandidate::LocalPath {
                path: fixture.root.join("missing.md"),
                source: HoverCandidateSource::ValidationFixture,
            })
            .expect_err("missing path should be rejected");
        assert!(matches!(missing, HoverCandidateRejection::MissingPath { .. }));

        let relative = filter
            .accept_candidate(HoverCandidate::LocalPath {
                path: PathBuf::from("relative.md"),
                source: HoverCandidateSource::ValidationFixture,
            })
            .expect_err("relative path should be rejected");
        assert!(matches!(relative, HoverCandidateRejection::RelativePath { .. }));

        let unsupported_extension = filter
            .accept_candidate(HoverCandidate::LocalPath {
                path: txt,
                source: HoverCandidateSource::ValidationFixture,
            })
            .expect_err("non-markdown file should be rejected");
        assert!(matches!(
            unsupported_extension,
            HoverCandidateRejection::UnsupportedExtension { .. }
        ));

        let unsupported_item = filter
            .accept_candidate(HoverCandidate::UnsupportedItem {
                description: "hovered widget was not a file row".to_string(),
                source: HoverCandidateSource::ValidationFixture,
            })
            .expect_err("unsupported item should be rejected");
        assert!(matches!(
            unsupported_item,
            HoverCandidateRejection::UnsupportedItem { .. }
        ));
    }
}
