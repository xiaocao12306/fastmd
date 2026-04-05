use std::fmt;
use std::path::{Path, PathBuf};

/// Source used to produce a hovered Explorer candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoverCandidateSource {
    ExplorerUiAutomation,
    ExplorerShellItem,
    ValidationFixture,
}

/// Candidate item surfaced by Explorer-specific integration code.
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

    pub fn source(&self) -> &HoverCandidateSource {
        &self.source
    }
}

/// Reasons a hovered Explorer candidate does not satisfy the macOS-matching
/// file acceptance rules yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoverCandidateRejection {
    UnsupportedItem {
        description: String,
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
            Self::MissingPath { path, source } => {
                write!(
                    f,
                    "missing hovered path from {:?}: {}",
                    source,
                    path.display()
                )
            }
            Self::Directory { path, source } => {
                write!(
                    f,
                    "hovered directory rejected from {:?}: {}",
                    source,
                    path.display()
                )
            }
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
pub struct WindowsMarkdownFilter;

impl WindowsMarkdownFilter {
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
        HoverCandidate, HoverCandidateRejection, HoverCandidateSource, WindowsMarkdownFilter,
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
                "fastmd-platform-windows-{nonce}-{}",
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
        let filter = WindowsMarkdownFilter;

        let accepted = filter
            .accept_candidate(HoverCandidate::LocalPath {
                path: path.clone(),
                source: HoverCandidateSource::ValidationFixture,
            })
            .expect("markdown file should be accepted");

        assert_eq!(accepted.path(), path.as_path());
        assert_eq!(accepted.source(), &HoverCandidateSource::ValidationFixture);
    }

    #[test]
    fn rejects_directories_even_if_the_name_looks_like_markdown() {
        let fixture = TempFixture::new();
        let path = fixture.create_directory("folder.md");
        let filter = WindowsMarkdownFilter;

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
    fn rejects_non_markdown_files() {
        let fixture = TempFixture::new();
        let path = fixture.write_file("notes.txt", "plain text");
        let filter = WindowsMarkdownFilter;

        let rejection = filter
            .accept_candidate(HoverCandidate::LocalPath {
                path: path.clone(),
                source: HoverCandidateSource::ValidationFixture,
            })
            .expect_err("non-markdown files should be rejected");

        assert_eq!(
            rejection,
            HoverCandidateRejection::UnsupportedExtension {
                path,
                extension: Some(String::from("txt")),
                source: HoverCandidateSource::ValidationFixture,
            }
        );
    }

    #[test]
    fn rejects_non_file_candidates_before_path_checks() {
        let filter = WindowsMarkdownFilter;

        let rejection = filter
            .accept_candidate(HoverCandidate::UnsupportedItem {
                description: String::from("Explorer tree expander"),
                source: HoverCandidateSource::ExplorerUiAutomation,
            })
            .expect_err("non-file candidate should be rejected");

        assert_eq!(
            rejection,
            HoverCandidateRejection::UnsupportedItem {
                description: String::from("Explorer tree expander"),
                source: HoverCandidateSource::ExplorerUiAutomation,
            }
        );
    }
}
