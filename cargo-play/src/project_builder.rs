use crate::Project;

use std::fs;
use std::path::PathBuf;

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
        // TODO: Infer dependencies from source code here

        let edition = self.project.edition;
        let id = self.project.hash;
        let dependencies = "";

        format!(
            r#"[package]
name = "p{id}"
version = "0.1.0"
edition = "{edition}"

[dependencies]
{dependencies}
"#
        )
    }

    pub fn copy(project: &'a mut Project<'b>) -> Result<(), ProjectBuildError> {
        let builder = ProjectBuilder::new(project);

        let cargo_config = builder.create_cargo_toml();

        let hash = builder.project.hash;
        let name = builder.project.target_prefix.unwrap_or("cargo-play");

        let folder_name = format!("{name}.{hash}");

        let target_dir = std::env::var("CARGO_BUILD_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("rust").join(folder_name));

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
