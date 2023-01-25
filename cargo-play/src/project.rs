use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Once;
use strum_macros::{Display, IntoStaticStr};
use thiserror::Error;

use crate::cargo_command_builder::CargoCommandBuilder;
use crate::project_builder::{ProjectBuildError, ProjectBuilder};

#[derive(Debug, Clone, Copy, Default, IntoStaticStr, PartialEq, Display)]
pub enum Edition {
    #[strum(to_string = "2015")]
    E2015,
    #[strum(to_string = "2018")]
    E2018,
    #[default]
    #[strum(to_string = "2021")]
    E2021,
}

#[derive(Debug, Clone, Copy, Default, IntoStaticStr, PartialEq)]
pub enum Subcommand {
    // Run the proigram
    #[default]
    #[strum(to_string = "run")]
    Run,
    // Just build the code (do nothing else)
    #[strum(to_string = "build")]
    Build,
    // Run tests
    #[strum(to_string = "test")]
    Test,
    // Show asm output
    #[strum(to_string = "rustc")]
    ASM,
    // Expand into macros - requires cargo-expand command be installed
    #[strum(to_string = "expand")]
    Expand,
    // Check for UB
    #[strum(to_string = "miri")]
    Miri,
    // Check code
    #[strum(to_string = "check")]
    Check,
    // Check against linter
    #[strum(to_string = "clippy")]
    Clippy,
    // Run code formatter
    #[strum(to_string = "fmt")]
    Rustfmt,
}

#[derive(Debug, Clone, Copy, Default, IntoStaticStr, PartialEq)]
pub enum Channel {
    #[default]
    #[strum(to_string = "stable")]
    Stable,
    #[strum(to_string = "beta")]
    Beta,
    #[strum(to_string = "nightly")]
    Nightly,
}

#[derive(Debug, Clone, Copy, Default, IntoStaticStr, PartialEq)]
pub enum Backtrace {
    #[default]
    #[strum(to_string = "")]
    None,
    #[strum(to_string = "1")]
    Short,
    #[strum(to_string = "full")]
    Full,
}

#[derive(Debug, Clone, Copy, Default, IntoStaticStr, PartialEq)]
pub enum BuildType {
    #[default]
    #[strum(to_string = "")]
    Debug,
    #[strum(to_string = "--release")]
    Release,
}

#[derive(Debug, Clone, Copy)]
pub struct File<'a> {
    pub(crate) name: &'a str,
    pub(crate) code: &'a str,
}

impl<'a> File<'a> {
    pub fn new(name: &'a str, code: &'a str) -> Self {
        Self { name, code }
    }
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("Failed to build project")]
    ProjectBuildError(#[from] ProjectBuildError),
}

#[derive(Debug, Default, Clone)]
pub struct Project<'a> {
    pub(crate) files: Vec<File<'a>>,
    pub(crate) hash: u64,
    pub(crate) edition: Edition,
    env: Vec<(&'a str, &'a str)>,
    cargo_command_builder: CargoCommandBuilder<'a>,
    pub(crate) location: Option<String>,
    pub(crate) target_prefix: Option<&'a str>,
}

impl<'a> Project<'a> {
    /// Create a new Project builder. Must have a unique hashable ID. This hashable ID identifies
    /// if a project uses the same source directory or not.
    pub fn new(hashable: impl Hash) -> Self {
        let mut hasher = DefaultHasher::new();
        hashable.hash(&mut hasher);
        let hash = hasher.finish();

        Self {
            hash,
            ..Default::default()
        }
    }

    // Set a source file (append)
    pub fn file(&mut self, file: File<'a>) -> &mut Self {
        self.files.push(file);
        self
    }

    /// Set the files (appends slice)
    pub fn files(&mut self, files: &[File<'a>]) -> &mut Self {
        self.files.extend_from_slice(files);
        self
    }

    /// Set the toolchain channel to use
    pub fn channel(&mut self, channel: Channel) -> &mut Self {
        self.cargo_command_builder.channel(channel);
        self
    }

    /// Set the cargo flag to be used in cargo command (append flag)
    pub fn cargo_flag(&mut self, flag: &'a str) -> &mut Self {
        self.cargo_command_builder.cargo_flag(flag);
        self
    }

    /// Set the cargo flags to be used in cargo command (append slice of flags)
    pub fn cargo_flags(&mut self, flags: &[&'a str]) -> &mut Self {
        self.cargo_command_builder.cargo_flags(flags);
        self
    }

    /// Set the cargo command to execute
    pub fn subcommand(&mut self, subcommand: Subcommand) -> &mut Self {
        self.cargo_command_builder.subcommand(subcommand);
        self
    }

    // Set a subcommand flag passed in cargo command (append flag)
    pub fn subcommand_flag(&mut self, flag: &'a str) -> &mut Self {
        self.cargo_command_builder.subcommand_flag(flag);
        self
    }

    /// Set the subcommand flags passed in cargo command (append slice of flags)
    pub fn subcommand_flags(&mut self, flags: &[&'a str]) -> &mut Self {
        self.cargo_command_builder.subcommand_flags(flags);
        self
    }

    /// Set the build type of cargo project
    pub fn build_type(&mut self, build_type: BuildType) -> &mut Self {
        self.cargo_command_builder.build_type(build_type);
        self
    }

    /// Append dash arg to cargo command
    pub fn dash_arg(&mut self, arg: &'a str) -> &mut Self {
        self.cargo_command_builder.dash_arg(arg);
        self
    }

    /// Append a slice of dash args to cargo command
    pub fn dash_args(&mut self, args: &[&'a str]) -> &mut Self {
        self.cargo_command_builder.dash_args(args);
        self
    }

    /// Set cargo edition
    pub fn edition(&mut self, edition: Edition) -> &mut Self {
        self.edition = edition;
        self
    }

    /// Set backtracing functionality
    pub fn backtrace(&mut self, backtrace: Backtrace) -> &mut Self {
        if backtrace == Backtrace::None {
            self.remove_env_var("RUST_BACKTRACE");
            return self;
        }

        self.env_var("RUST_BACKTRACE", backtrace.into())
    }

    /// sets rustflags env var (replaces if exists)
    /// Shorthand for `project.env_var("RUSTFLAGS", "val");`
    pub fn rust_flags(&mut self, val: &'a str) -> &mut Self {
        self.env_var("RUSTFLAGS", val)
    }

    /// Sets an env var (replaces var if it exists)
    pub fn env_var(&mut self, var: &'a str, val: &'a str) -> &mut Self {
        let index = self.env.iter().position(|i| i.0 == var);
        if let Some(i) = index {
            self.env[i] = (var, val);
        } else {
            self.env.push((var, val));
        }

        self
    }

    // Sets a bunch of env vars
    pub fn env_vars(&mut self, vars: &[(&'a str, &'a str)]) -> &mut Self {
        for (var, val) in vars.iter() {
            self.env_var(var, val);
        }

        self
    }

    /// Remove env var from list
    pub fn remove_env_var(&mut self, var: &str) {
        let index = self.env.iter().position(|i| i.0 == var);
        if let Some(i) = index {
            self.env.remove(i);
        }
    }

    /// Prefix to use for target folder name. E.g, instead of `cargo-play.<id>`, use `<prefix>.<id>`
    pub fn target_prefix(&mut self, prefix: &'a str) -> &mut Self {
        self.target_prefix = Some(prefix);
        self
    }

    /// Cargo clean the project. If project wasn't created yet, returns None
    /// TODO: Make lib that can pipe stdout and stderr together
    pub fn clean_project(&mut self) -> Option<Child> {
        let child = Command::new("cargo")
            .arg("clean")
            .current_dir(self.location.as_ref()?)
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .spawn()
            .unwrap();

        Some(child)
    }

    /// Create the project and return the command
    pub fn create(&mut self) -> Result<Command, ProjectError> {
        // Make sure you actually put a subcommand in before creating it
        assert!(self.cargo_command_builder.subcommand.is_some());

        // Cargo likes to - for some reason - put toolchain cargo paths first in the PATH
        // these cargo binaries DO NOT support "+toolchain" format, and we must remove them from PATH
        // These are set on the main parent and gets inherited in the child process
        //
        // The most recognizable part of the paths are:
        // - they end in lib or bin
        // - the path has .rustup/toolchains, in it
        static FIX_PATHS: Once = Once::new();
        FIX_PATHS.call_once(|| {
            const ENV_PATH_SEP: &str = if cfg!(target_os = "windows") {
                ";"
            } else {
                ":"
            };

            let paths = std::env::var("PATH").unwrap_or_default();

            let reconstituted_paths: Vec<String> = paths
                .split(ENV_PATH_SEP)
                .filter(|path| {
                    let path_buffer = PathBuf::from(path);
                    if !path_buffer.ends_with("lib") && !path_buffer.ends_with("bin") {
                        true
                    } else {
                        let mut ancestors = path_buffer.ancestors();
                        !ancestors.any(|ancestor_path| {
                            let ancestor = ancestor_path
                                .file_name()
                                .unwrap_or_default()
                                .to_str()
                                .unwrap();

                            let ancestor_parent = ancestor_path
                                .parent()
                                .unwrap_or_else(|| Path::new(""))
                                .file_name()
                                .unwrap_or_default()
                                .to_str()
                                .unwrap();

                            ancestor == "toolchains" && ancestor_parent == ".rustup"
                        })
                    }
                })
                .map(|path| path.to_string())
                .collect();

            std::env::remove_var("PATH");
            std::env::set_var("PATH", reconstituted_paths.join(ENV_PATH_SEP));
        });

        let mut command = self.cargo_command_builder.build();
        command.envs(self.env.clone());

        // Copy and create project in the filesystem
        ProjectBuilder::copy(self)?;

        command.current_dir(self.location.as_ref().unwrap());

        Ok(command)
    }
}
