use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    http::StatusCode,
};
use askama::Template;
use crate::AppState;
use rune_core::object::{ObjectId, Object};
use rune_core::diff::{diff_trees, diff_blobs_structured, DiffStatus, DiffTag};
use std::str::FromStr;

#[derive(Template)]
#[template(path = "diff.html")]
pub struct DiffTemplate {
    pub repo_name: String,
    pub repo_path: String,
    pub commit_hash: String,
    pub parent_hash: Option<String>,
    pub diffs: Vec<FileDiffView>,
    pub summary_message: String,
}

pub struct FileDiffView {
    pub path: String,
    pub status: String,
    pub is_binary: bool,
    pub too_large: bool,
    pub lines: Vec<DiffLineView>,
}

pub struct DiffLineView {
    pub class: String,
    pub sign: String,
    pub content: String,
}

const MAX_DIFF_FILE_SIZE: usize = 500 * 1024; // 500 KB limit for diff rendering

pub async fn handler(
    State(state): State<AppState>,
    Path(hash): Path<String>,
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

    let parent_hash = commit_obj.parent_hashes.first().copied();
    
    let parent_tree_hash = if let Some(parent) = parent_hash {
        if let Ok(Object::Commit(p_commit)) = store.read(&parent) {
            Some(p_commit.tree_hash)
        } else {
            None
        }
    } else {
        None
    };

    let tree_diff = match diff_trees(&store, parent_tree_hash.as_ref(), &commit_obj.tree_hash) {
        Ok(d) => d,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to compute diff: {}", e)).into_response(),
    };

    let mut file_diffs = Vec::new();
    let mut sorted_changes: Vec<_> = tree_diff.changes.into_iter().collect();
    sorted_changes.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by path

    for (path, status) in sorted_changes {
        let (old_obj_hash, new_obj_hash, status_str) = match &status {
            DiffStatus::Added(h) => (None, Some(*h), "Added"),
            DiffStatus::Deleted(h) => (Some(*h), None, "Deleted"),
            DiffStatus::Modified { old, new } => (Some(*old), Some(*new), "Modified"),
        };
        
        let mut is_binary = false;
        let mut too_large = false;
        let mut lines = Vec::new();
        
        // Read old content to check size and binary
        let old_content = old_obj_hash.and_then(|h| store.read(&h).ok()).and_then(|obj| {
            if let Object::Blob(b) = obj { Some(b.content) } else { None }
        }).unwrap_or_default();
        
        // Read new content to check size and binary
        let new_content = new_obj_hash.and_then(|h| store.read(&h).ok()).and_then(|obj| {
            if let Object::Blob(b) = obj { Some(b.content) } else { None }
        }).unwrap_or_default();
        
        if old_content.len() > MAX_DIFF_FILE_SIZE || new_content.len() > MAX_DIFF_FILE_SIZE {
            too_large = true;
        } else {
            // Check for binary
            let sniff_old = old_content[..std::cmp::min(1024, old_content.len())].contains(&0);
            let sniff_new = new_content[..std::cmp::min(1024, new_content.len())].contains(&0);
            
            if sniff_old || sniff_new {
                is_binary = true;
            } else {
                // Compute structured diff
                if let Ok(diff_lines) = diff_blobs_structured(&store, old_obj_hash.as_ref(), new_obj_hash.as_ref()) {
                    for line in diff_lines {
                        let (class, sign) = match line.tag {
                            DiffTag::Insert => ("bg-add-bg text-add-text", "+"),
                            DiffTag::Delete => ("bg-delete-bg text-delete-text", "-"),
                            DiffTag::Equal => ("text-text-primary", " "),
                        };
                        lines.push(DiffLineView {
                            class: class.to_string(),
                            sign: sign.to_string(),
                            content: line.content.clone(),
                        });
                    }
                }
            }
        }
        
        file_diffs.push(FileDiffView {
            path,
            status: status_str.to_string(),
            is_binary,
            too_large,
            lines,
        });
    }

    let summary_message = format!("{} files changed", file_diffs.len());

    let template = DiffTemplate {
        repo_name,
        repo_path: state.repo_path.display().to_string(),
        commit_hash: hash,
        parent_hash: parent_hash.map(|h| h.to_string()),
        diffs: file_diffs,
        summary_message,
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e))).into_response()
}
