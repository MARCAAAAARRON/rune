use axum::{
    extract::{Query, State},
    response::Html,
};
use askama::Template;
use serde::Deserialize;
use std::time::{UNIX_EPOCH, Duration};
use chrono::{DateTime, Utc};
use crate::AppState;

#[derive(Deserialize)]
pub struct Pagination {
    page: Option<usize>,
    branch: Option<String>,
}

pub struct CommitView {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub date_str: String,
}

#[derive(Template)]
#[template(path = "commits.html")]
pub struct CommitsTemplate {
    pub repo_name: String,
    pub repo_path: String,
    pub current_branch: Option<String>,
    pub commits: Vec<CommitView>,
    pub current_page: usize,
    pub has_prev: bool,
    pub has_next: bool,
}

const PAGE_SIZE: usize = 50;

pub async fn handler(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Html<String> {
    let repo_path = state.repo_path.clone();
    let repo_name = repo_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let repo_path_str = repo_path.display().to_string();

    let page = pagination.page.unwrap_or(1).max(1);
    let branch = pagination.branch.clone();
    
    // Get full log
    let log = if let Some(ref branch_name) = branch {
        rune_core::log_from_ref(&repo_path, branch_name).unwrap_or_default()
    } else {
        rune_core::log(&repo_path).unwrap_or_default()
    };
    
    let total_commits = log.len();
    let total_pages = (total_commits + PAGE_SIZE - 1) / PAGE_SIZE;
    let actual_page = if page > total_pages { total_pages.max(1) } else { page };
    
    let start_idx = (actual_page - 1) * PAGE_SIZE;
    let end_idx = (start_idx + PAGE_SIZE).min(total_commits);
    
    let slice = if start_idx < total_commits {
        &log[start_idx..end_idx]
    } else {
        &[]
    };
    
    let commit_views = slice.iter().map(|(hash, commit)| {
        let hash_str = hash.to_string();
        let short_hash = hash_str.chars().take(7).collect();
        
        let d = UNIX_EPOCH + Duration::from_secs(commit.timestamp);
        let datetime: DateTime<Utc> = d.into();
        let date_str = datetime.format("%b %d, %Y %H:%M").to_string();
        
        CommitView {
            hash: hash_str,
            short_hash,
            message: commit.message.clone(),
            author: commit.author.clone(),
            date_str,
        }
    }).collect();
    
    let template = CommitsTemplate {
        repo_name,
        repo_path: repo_path_str,
        current_branch: branch,
        commits: commit_views,
        current_page: actual_page,
        has_prev: actual_page > 1,
        has_next: actual_page < total_pages,
    };
    
    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}
