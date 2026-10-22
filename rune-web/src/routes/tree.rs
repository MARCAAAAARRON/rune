use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect},
    http::StatusCode,
};
use askama::Template;
use crate::AppState;
use rune_core::object::{ObjectId, Object, FileMode};
use std::str::FromStr;

#[derive(Template)]
#[template(path = "tree.html")]
pub struct TreeTemplate {
    pub repo_name: String,
    pub repo_path: String,
    pub commit_hash: String,
    pub current_path: String,
    pub entries: Vec<TreeEntryView>,
}

pub struct TreeEntryView {
    pub name: String,
    pub is_dir: bool,
    pub path: String, // Full path for links
}

pub async fn root_handler(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> impl IntoResponse {
    handler(State(state), Path((hash, "".to_string()))).await
}

pub async fn head_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo_path = state.repo_path.clone();
    
    // Get the HEAD commit hash
    let head_hash = if let Ok(log) = rune_core::log(&repo_path) {
        if let Some((hash, _)) = log.first() {
            Some(hash.to_string())
        } else {
            None
        }
    } else {
        None
    };
    
    if let Some(hash) = head_hash {
        Redirect::to(&format!("/commit/{}/tree", hash)).into_response()
    } else {
        (StatusCode::NOT_FOUND, "No commits found").into_response()
    }
}

pub async fn handler(
    State(state): State<AppState>,
    Path((hash, path)): Path<(String, String)>,
) -> impl IntoResponse {
    let repo_path = state.repo_path.clone();
    let repo_name = repo_path.file_name().unwrap_or_default().to_string_lossy().to_string();

    let store = rune_core::store::ObjectStore::new(&repo_path);
    
    let commit_id = match ObjectId::from_str(&hash) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid commit hash".to_string()).into_response(),
    };

    let commit_obj = match store.read(&commit_id) {
        Ok(Object::Commit(c)) => c,
        Ok(_) => return (StatusCode::BAD_REQUEST, "Hash is not a commit".to_string()).into_response(),
        Err(_) => return (StatusCode::NOT_FOUND, "Commit not found".to_string()).into_response(),
    };

    // Safely resolve the path
    let (obj, _mode) = match rune_core::tree_walk::resolve_path(&store, &commit_obj.tree_hash, &path) {
        Ok(res) => res,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Security violation") || msg.contains("not allowed") {
                return (StatusCode::FORBIDDEN, msg).into_response();
            } else {
                return (StatusCode::NOT_FOUND, msg).into_response();
            }
        }
    };

    let tree = match obj {
        Object::Tree(t) => t,
        _ => {
            // If it's a blob, redirect to blob route
            let blob_url = format!("/commit/{}/blob/{}", hash, path);
            return Redirect::to(&blob_url).into_response();
        }
    };

    let mut entries = Vec::new();
    
    // Add parent directory link if we're not at root
    if !path.is_empty() && path != "/" {
        let parent_path = if let Some(idx) = path.trim_end_matches('/').rfind('/') {
            &path[..idx]
        } else {
            ""
        };
        entries.push(TreeEntryView {
            name: "..".to_string(),
            is_dir: true,
            path: parent_path.to_string(),
        });
    }

    let mut sorted_entries = tree.entries;
    sorted_entries.sort_by(|a, b| {
        // Directories first, then alphabetical
        let a_is_dir = a.mode == FileMode::Directory;
        let b_is_dir = b.mode == FileMode::Directory;
        b_is_dir.cmp(&a_is_dir).then(a.name.cmp(&b.name))
    });

    for entry in sorted_entries {
        let is_dir = entry.mode == FileMode::Directory;
        let entry_path = if path.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", path.trim_end_matches('/'), entry.name)
        };
        
        entries.push(TreeEntryView {
            name: entry.name,
            is_dir,
            path: entry_path,
        });
    }

    let template = TreeTemplate {
        repo_name,
        repo_path: state.repo_path.display().to_string(),
        commit_hash: hash,
        current_path: path,
        entries,
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e))).into_response()
}
