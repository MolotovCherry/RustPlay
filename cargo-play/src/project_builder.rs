use crate::infer::infer_deps;
use crate::Project;

use std::fs;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectBuildError {
    #[error("Io error occurred")]
    Io(#[from] std::io::Error),
}

pub struct ProjectBuilder<'a, 'b> {
    project: &'a mut Project<'b>,
}

impl<'a, 'b> ProjectBuilder<'a, 'b> {
    fn new(project: &'a mut Project<'b>) -> Self {
        Self { project }
    }

    fn create_cargo_toml(&self) -> String {
        let edition = self.project.edition;
        let id = self.project.hash;
        // if the user has malformed code, or wrong deps that's not our fault. Running cargo will reveal it
        let _ = infer_deps(&self.project.files);
        let dependencies = infer_deps(&self.project.files).unwrap_or_default();

        // we can add extra cargo toml, but only in the main file
        let mut extra_cargo = String::new();
        let main_file = self
            .project
            .files
            .iter()
            .find(|f| f.name == "main")
            .expect("Main file not found");

        for l in main_file.code.lines() {
            if l.starts_with("//> ") {
                extra_cargo.push_str(&format!("{}\n", l.replace("//> ", "")));
                continue;
            } else if l.starts_with("//# ") {
                // just ignore these lines
                continue;
            }

            break;
        }

        let mut formatted = format!(
            r#"[package]
name = "p{id}"
version = "0.1.0"
edition = "{edition}"

[dependencies]
{dependencies}
"#
        );

        if !extra_cargo.is_empty() {
            formatted.push_str(&format!("\n{}", extra_cargo));
        }

        formatted
    }

    pub fn copy(project: &'a mut Project<'b>) -> Result<(), ProjectBuildError> {
        let builder = ProjectBuilder::new(project);

        let cargo_config = builder.create_cargo_toml();

        let hash = builder.project.hash;
        let name = builder.project.target_prefix.unwrap_or("cargo-play");

        let folder_name = format!("{name}.{hash}");

        let target_dir = std::env::temp_dir().join("rust").join(folder_name);

        // create all directories straight to src
        let target_dir_src = target_dir.join("src");
        if !target_dir_src.exists() {
            fs::create_dir_all(&target_dir_src)?;
        }

        fs::write(target_dir.join("Cargo.toml"), cargo_config)?;

        for file in &builder.project.files {
            fs::write(target_dir_src.join(format!("{}.rs", file.name)), file.code)?;
        }

        builder.project.location = Some(target_dir.to_str().unwrap().to_string());

        Ok(())
    }
}
