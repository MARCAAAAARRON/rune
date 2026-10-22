use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use askama::Template;
use crate::AppState;

#[derive(Template)]
#[template(path = "refs.html")]
pub struct RefsTemplate {
    pub repo_name: String,
    pub repo_path: String,
    pub current_branch: Option<String>,
    pub local_branches: Vec<String>,
    pub remote_branches: Vec<RemoteBranchView>,
}

pub struct RemoteBranchView {
    pub remote: String,
    pub branch: String,
}

pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    let repo_path = state.repo_path.clone();
    let repo_name = repo_path.file_name().unwrap_or_default().to_string_lossy().to_string();

    let head = rune_core::refs::read_head(&repo_path).ok();
    let current_branch = match head {
        Some(rune_core::refs::HeadRef::Branch(b)) => Some(b),
        _ => None,
    };

    let local_branches = rune_core::refs::list_refs(&repo_path).unwrap_or_default();
    
    let remote_refs = rune_core::refs::list_remote_refs(&repo_path).unwrap_or_default();
    let remote_branches = remote_refs.into_iter().map(|(remote, branch)| {
        RemoteBranchView { remote, branch }
    }).collect();

    let template = RefsTemplate {
        repo_name,
        repo_path: state.repo_path.display().to_string(),
        current_branch,
        local_branches,
        remote_branches,
    };

    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e))).into_response()
}
