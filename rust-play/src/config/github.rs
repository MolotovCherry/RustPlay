use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::mpsc::{channel, Receiver};
use thiserror::Error;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GitHub {
    pub access_token: String,
}

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("No access token found")]
    NoAuthentication,
    #[error("Forbidden")]
    Forbidden,
    #[error("Resource not found")]
    NotFound,
    #[error("Validation failed, or the endpoint has been spammed.")]
    ValidationFailed,
    #[error("Unknnown error occurred")]
    Unknown,
}

impl GitHub {
    /// Creates a new github gist using a title and content
    /// Does not block, but instead returns a receiver you can use to receive it
    pub fn create_gist(&self, content: &str) -> Receiver<Result<String, GitHubError>> {
        let (tx, rx) = channel();

        // Error out immediately if no access token was provided
        if self.access_token.is_empty() {
            let _ = tx.send(Err(GitHubError::NoAuthentication));
            return rx;
        }

        let access_token = self.access_token.clone();
        let content = content.to_owned();

        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::new();

            let body = json!({
                "description": "Created by Rust Play <https://github.com/MolotovCherry/RustPlay>",
                "public": true,
                "files": {
                    "playground.rs": {"content": content}
                }
            })
            .to_string();

            let result = client
                .post("https://api.github.com/gists")
                .header("User-Agent", "RustPlay")
                .header("accept", "application/vnd.github+json")
                .bearer_auth(access_token)
                .body(body)
                .send();

            let reply = match result {
                Ok(v) => v,
                Err(e) => {
                    if e.is_status() {
                        let code = e.status().unwrap().as_u16();
                        let error = match code {
                            403 => GitHubError::Forbidden,
                            404 => GitHubError::NotFound,
                            422 => GitHubError::ValidationFailed,
                            _ => GitHubError::Unknown,
                        };

                        let _ = tx.send(Err(error));
                        return;
                    }

                    let _ = tx.send(Err(GitHubError::Unknown));
                    return;
                }
            };

            let reply = serde_json::from_str::<GitHubReply>(&reply.text().unwrap())
                .expect("Failed to unwrap github reply");

            let _ = tx.send(Ok(reply.id));
        });

        rx
    }
}

#[derive(Debug, Deserialize)]
struct GitHubReply {
    id: String,
}
