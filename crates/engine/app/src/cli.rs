//! Command line argument processing for Crossworld applications
//!
//! This module provides a shared CLI argument structure using clap that all
//! crates using the app framework can extend.
//!
//! # Usage
//!
//! Applications can use [`CommonArgs`] directly or extend it with additional arguments:
//!
//! ```ignore
//! use app::cli::CommonArgs;
//! use clap::Parser;
//!
//! #[derive(Parser)]
//! struct MyAppArgs {
//!     #[command(flatten)]
//!     common: CommonArgs,
//!
//!     /// My app-specific flag
//!     #[arg(long)]
//!     my_flag: bool,
//! }
//!
//! fn main() {
//!     let args = MyAppArgs::parse();
//!     let config = args.common.apply_to(AppConfig::new("My App"))?;
//!     // ...
//! }
//! ```

use clap::Args;
use std::path::PathBuf;

/// Common command line arguments shared by all Crossworld applications
///
/// These arguments are available in any application using the app framework.
/// Use `#[command(flatten)]` to include these in your own argument struct.
#[derive(Args, Debug, Clone, Default)]
pub struct CommonArgs {
    /// Run N frames with debug output then exit
    #[arg(long, value_name = "FRAMES")]
    pub debug: Option<u64>,

    /// Display a note overlay with the given message (supports markdown)
    #[arg(long, short = 'n', value_name = "MESSAGE")]
    pub note: Option<String>,

    /// Display a review panel with the given markdown message
    #[arg(long, short = 'r', value_name = "MESSAGE")]
    pub review: Option<String>,

    /// Display a review panel with markdown content loaded from a file
    #[arg(long, value_name = "PATH")]
    pub review_file: Option<PathBuf>,

    /// Load scene configuration from a Lua file
    #[arg(long, short = 'c', value_name = "PATH")]
    pub config: Option<PathBuf>,
}

impl CommonArgs {
    /// Apply the common arguments to an AppConfig
    ///
    /// Returns an error if the review file cannot be read.
    #[cfg(feature = "runtime")]
    pub fn apply_to(
        &self,
        mut config: crate::AppConfig,
    ) -> Result<crate::AppConfig, std::io::Error> {
        if let Some(frames) = self.debug {
            config = config.with_debug_mode(frames);
        }

        if let Some(ref note) = self.note {
            config = config.with_note(note.clone());
        }

        if let Some(ref message) = self.review {
            config = config.with_review_text(message.clone());
        }

        if let Some(ref path) = self.review_file {
            config = config.with_review_file(path.clone())?;
        }

        Ok(config)
    }

    /// Get the config path if specified
    pub fn config_path(&self) -> Option<&PathBuf> {
        self.config.as_ref()
    }

    /// Check if debug mode is enabled
    pub fn is_debug(&self) -> bool {
        self.debug.is_some()
    }

    /// Get the number of debug frames
    pub fn debug_frames(&self) -> Option<u64> {
        self.debug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestArgs {
        #[command(flatten)]
        common: CommonArgs,
    }

    #[test]
    fn test_no_args() {
        let args = TestArgs::parse_from(["test"]);
        assert!(args.common.debug.is_none());
        assert!(args.common.note.is_none());
        assert!(args.common.review.is_none());
        assert!(args.common.review_file.is_none());
        assert!(args.common.config.is_none());
    }

    #[test]
    fn test_debug_arg() {
        let args = TestArgs::parse_from(["test", "--debug", "100"]);
        assert_eq!(args.common.debug, Some(100));
    }

    #[test]
    fn test_note_arg() {
        let args = TestArgs::parse_from(["test", "--note", "Hello world"]);
        assert_eq!(args.common.note.as_deref(), Some("Hello world"));
    }

    #[test]
    fn test_note_short_arg() {
        let args = TestArgs::parse_from(["test", "-n", "Hello world"]);
        assert_eq!(args.common.note.as_deref(), Some("Hello world"));
    }

    #[test]
    fn test_review_arg() {
        let args = TestArgs::parse_from(["test", "--review", "This is a review message"]);
        assert_eq!(
            args.common.review.as_deref(),
            Some("This is a review message")
        );
    }

    #[test]
    fn test_review_short_arg() {
        let args = TestArgs::parse_from(["test", "-r", "Short review"]);
        assert_eq!(args.common.review.as_deref(), Some("Short review"));
    }

    #[test]
    fn test_review_file_arg() {
        let args = TestArgs::parse_from(["test", "--review-file", "doc/review.md"]);
        assert_eq!(
            args.common.review_file,
            Some(PathBuf::from("doc/review.md"))
        );
    }

    #[test]
    fn test_config_arg() {
        let args = TestArgs::parse_from(["test", "--config", "scene.lua"]);
        assert_eq!(args.common.config, Some(PathBuf::from("scene.lua")));
    }

    #[test]
    fn test_config_short_arg() {
        let args = TestArgs::parse_from(["test", "-c", "scene.lua"]);
        assert_eq!(args.common.config, Some(PathBuf::from("scene.lua")));
    }

    #[test]
    fn test_all_args() {
        let args = TestArgs::parse_from([
            "test",
            "--debug",
            "50",
            "--note",
            "Testing",
            "--review",
            "Review message",
            "--review-file",
            "review.md",
            "--config",
            "config.lua",
        ]);
        assert_eq!(args.common.debug, Some(50));
        assert_eq!(args.common.note.as_deref(), Some("Testing"));
        assert_eq!(args.common.review.as_deref(), Some("Review message"));
        assert_eq!(args.common.review_file, Some(PathBuf::from("review.md")));
        assert_eq!(args.common.config, Some(PathBuf::from("config.lua")));
    }
}
