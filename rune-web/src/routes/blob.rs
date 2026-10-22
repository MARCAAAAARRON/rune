use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    http::StatusCode,
};
use askama::Template;
use crate::AppState;
use rune_core::object::{ObjectId, Object};
use std::str::FromStr;

use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;

#[derive(Template)]
#[template(path = "blob.html")]
pub struct BlobTemplate {
    pub repo_name: String,
    pub repo_path: String,
    pub commit_hash: String,
    pub current_path: String,
    pub is_binary: bool,
    pub size_bytes: usize,
    pub too_large: bool,
    pub highlighted_html: String,
}

const MAX_BLOB_RENDER_SIZE: usize = 2 * 1024 * 1024; // 2 MB limit

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

    let (obj, _mode) = match rune_core::tree_walk::resolve_path(&store, &commit_obj.tree_hash, &path) {
        Ok(res) => res,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Security violation") {
                return (StatusCode::FORBIDDEN, msg).into_response();
            }
            return (StatusCode::NOT_FOUND, msg).into_response();
        }
    };

    let blob = match obj {
        Object::Blob(b) => b,
        Object::Tree(_) => return (StatusCode::BAD_REQUEST, "Path is a directory, not a blob".to_string()).into_response(),
        _ => return (StatusCode::BAD_REQUEST, "Invalid object type".to_string()).into_response(),
    };

    let size_bytes = blob.content.len();
    
    // Check if binary (sniff first 1024 bytes for null byte)
    let sniff_len = std::cmp::min(1024, size_bytes);
    let is_binary = blob.content[..sniff_len].contains(&0);
    
    let mut highlighted_html = String::new();
    let mut too_large = false;
    
    if size_bytes > MAX_BLOB_RENDER_SIZE {
        too_large = true;
    } else if !is_binary {
        let content_str = String::from_utf8_lossy(&blob.content);
        
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        
        // Use extension to find syntax
        let ext = std::path::Path::new(&path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
            
        let syntax = ps.find_syntax_by_extension(ext)
            .unwrap_or_else(|| ps.find_syntax_plain_text());
            
        let theme = &ts.themes["base16-ocean.dark"];
        
        // Generate highlighted HTML
        match highlighted_html_for_string(&content_str, &ps, syntax, theme) {
            Ok(html) => {
                // syntect generates inline styles and a <pre> tag.
                highlighted_html = html;
            }
            Err(_) => {
                // Fallback to plain text if highlighting fails
                let escaped = escape_html(&content_str);
                highlighted_html = format!("<pre><code>{}</code></pre>", escaped);
            }
        }
    }

    let template = BlobTemplate {
        repo_name,
        repo_path: state.repo_path.display().to_string(),
        commit_hash: hash,
        current_path: path,
        is_binary,
        size_bytes,
        too_large,
        highlighted_html,
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e))).into_response()
}

fn escape_html(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() + 10);
    for c in s.chars() {
        match c {
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(c),
        }
    }
    escaped
}
