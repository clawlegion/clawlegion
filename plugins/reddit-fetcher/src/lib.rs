//! Reddit Fetcher Plugin for ClawLegion
//!
//! This plugin provides Reddit content fetching capabilities via old.reddit.com API.

use async_trait::async_trait;
use clawlegion_core::{Error, PluginError, Result};
use clawlegion_plugin_sdk::{
    plugin, tool, Plugin, PluginContext, PluginMetadata, Tool, ToolContext, ToolMetadata,
    ToolResult, ToolVisibility,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Reddit post data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditPost {
    pub title: String,
    pub author: String,
    pub score: i64,
    pub url: String,
    pub permalink: String,
    pub created_utc: u64,
    pub num_comments: i64,
    pub selftext: String,
    pub thumbnail: String,
    pub subreddit: String,
    pub is_video: bool,
    pub over_18: bool,
}

/// Input for reddit fetch tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditFetchInput {
    /// Subreddit name (required)
    pub subreddit: String,
    /// Sort method: new, hot, top, controversial (default: new)
    #[serde(default = "default_sort")]
    pub sort: String,
    /// Time range: hour, day, week, month, year, all (default: all)
    #[serde(default = "default_time_range")]
    pub time_range: String,
    /// Limit results (default: 25, max: 100)
    #[serde(default = "default_limit")]
    pub limit: i32,
    /// Pagination token (optional)
    #[serde(default)]
    pub after: Option<String>,
}

fn default_sort() -> String {
    "new".to_string()
}

fn default_time_range() -> String {
    "all".to_string()
}

fn default_limit() -> i32 {
    25
}

/// Output for reddit fetch tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditFetchOutput {
    pub posts: Vec<RedditPost>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub count: usize,
}

/// Reddit Fetch Tool - Private tool for fetching Reddit content
pub struct RedditFetchTool {
    metadata: ToolMetadata,
    client: reqwest::Client,
}

impl RedditFetchTool {
    pub fn new() -> Self {
        let input_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "subreddit": {
                    "type": "string",
                    "description": "Subreddit name (e.g., 'rust', 'programming')"
                },
                "sort": {
                    "type": "string",
                    "enum": ["new", "hot", "top", "controversial"],
                    "default": "new",
                    "description": "Sort method"
                },
                "time_range": {
                    "type": "string",
                    "enum": ["hour", "day", "week", "month", "year", "all"],
                    "default": "all",
                    "description": "Time range filter (only applies to top/controversial)"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "default": 25,
                    "description": "Number of posts to fetch"
                },
                "after": {
                    "type": "string",
                    "nullable": true,
                    "description": "Pagination token for next page"
                }
            },
            "required": ["subreddit"]
        });

        Self {
            metadata: ToolMetadata {
                name: "reddit_fetch".to_string(),
                version: "1.0.0".to_string(),
                description: "Fetch posts from Reddit via old.reddit.com API. Supports sorting by new/hot/top/controversial with time range filtering.".to_string(),
                visibility: ToolVisibility::Private,
                tags: vec![
                    "reddit".to_string(),
                    "social".to_string(),
                    "content".to_string(),
                    "news".to_string(),
                ],
                input_schema,
                output_schema: None,
                requires_llm: false,
            },
            client: reqwest::Client::builder()
                .user_agent("ClawLegion-Reddit-Fetcher/0.1.0")
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn default_metadata() -> ToolMetadata {
        Self::new().metadata
    }

    async fn fetch_reddit(&self, input: RedditFetchInput) -> Result<RedditFetchOutput> {
        // Validate sort method
        let valid_sorts = ["new", "hot", "top", "controversial"];
        let sort = if valid_sorts.contains(&input.sort.as_str()) {
            input.sort
        } else {
            "new".to_string()
        };

        // Validate and build time range parameter
        let valid_time_ranges = ["hour", "day", "week", "month", "year", "all"];
        let time_range = if valid_time_ranges.contains(&input.time_range.as_str()) {
            input.time_range
        } else {
            "all".to_string()
        };

        // Build URL
        let limit = input.limit.clamp(1, 100);
        let mut url = format!(
            "https://old.reddit.com/r/{}/{}.json?limit={}",
            input.subreddit, sort, limit
        );

        // Add time range parameter for top/controversial sorting
        if sort == "top" || sort == "controversial" {
            url.push_str(&format!("&t={}", time_range));
        }

        // Add pagination
        if let Some(after) = &input.after {
            url.push_str(&format!("&after={}", after));
        }

        // Make request
        let response = self.client.get(&url).send().await.map_err(|e| {
            Error::Plugin(PluginError::LoadFailed(format!("Failed to fetch Reddit: {}", e)))
        })?;

        if !response.status().is_success() {
            return Err(Error::Plugin(PluginError::LoadFailed(format!(
                "Reddit API error: {}",
                response.status()
            ))));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| {
            Error::Plugin(PluginError::LoadFailed(format!(
                "Failed to parse Reddit response: {}",
                e
            )))
        })?;

        // Parse Reddit response
        let data = body
            .get("data")
            .ok_or_else(|| Error::Plugin(PluginError::LoadFailed("No data in Reddit response".to_string())))?;

        let children = data
            .get("children")
            .and_then(|c| c.as_array())
            .ok_or_else(|| Error::Plugin(PluginError::LoadFailed("No children in Reddit response".to_string())))?;

        let mut posts = Vec::new();
        for child in children {
            if let Some(post_data) = child.get("data") {
                if let Some(post) = parse_post(post_data, &input.subreddit) {
                    posts.push(post);
                }
            }
        }

        let after = data
            .get("after")
            .and_then(|a| a.as_str())
            .map(String::from);

        let before = data
            .get("before")
            .and_then(|b| b.as_str())
            .map(String::from);

        let count = posts.len();
        Ok(RedditFetchOutput {
            posts,
            after,
            before,
            count,
        })
    }
}

fn parse_post(data: &serde_json::Value, subreddit: &str) -> Option<RedditPost> {
    Some(RedditPost {
        title: data.get("title").and_then(|v| v.as_str())?.to_string(),
        author: data
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("[deleted]")
            .to_string(),
        score: data.get("score").and_then(|v| v.as_i64()).unwrap_or(0),
        url: data.get("url").and_then(|v| v.as_str())?.to_string(),
        permalink: data
            .get("permalink")
            .and_then(|v| v.as_str())?
            .to_string(),
        created_utc: data
            .get("created_utc")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        num_comments: data
            .get("num_comments")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        selftext: data
            .get("selftext")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        thumbnail: data
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        subreddit: data
            .get("subreddit")
            .and_then(|v| v.as_str())
            .unwrap_or(subreddit)
            .to_string(),
        is_video: data.get("is_video").and_then(|v| v.as_bool()).unwrap_or(false),
        over_18: data.get("over_18").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

#[async_trait]
impl Tool for RedditFetchTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, _ctx: &ToolContext, args: serde_json::Value) -> Result<ToolResult> {
        let input: RedditFetchInput = serde_json::from_value(args).map_err(|e| {
            Error::Plugin(PluginError::LoadFailed(format!("Invalid input: {}", e)))
        })?;

        match self.fetch_reddit(input).await {
            Ok(output) => Ok(ToolResult::success(serde_json::to_value(output).map_err(|e| {
                Error::Plugin(PluginError::LoadFailed(format!("Failed to serialize output: {}", e)))
            })?)),
            Err(e) => Ok(ToolResult::error(format!("{}", e))),
        }
    }
}

/// Reddit Fetcher Plugin
pub struct RedditFetcherPlugin {
    metadata: PluginMetadata,
    tool: Arc<RedditFetchTool>,
}

impl RedditFetcherPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "reddit-fetcher".to_string(),
                version: "0.1.0".to_string(),
                description: "Reddit content fetcher plugin - provides tools for fetching posts from old.reddit.com with customizable sorting and time range filters.".to_string(),
                author: "ClawLegion Team".to_string(),
                core_version: env!("CARGO_PKG_VERSION").to_string(),
                dependencies: vec![],
                tags: vec![
                    "reddit".to_string(),
                    "social".to_string(),
                    "content".to_string(),
                    "fetcher".to_string(),
                ],
            },
            tool: Arc::new(RedditFetchTool::new()),
        }
    }

    pub fn default_metadata() -> PluginMetadata {
        clawlegion_plugin_sdk::PluginBuilder::new("reddit-fetcher", "0.1.0")
            .description("Reddit content fetcher plugin with sorting and time range options")
            .author("ClawLegion Team")
            .tag("reddit")
            .tag("social")
            .tag("content")
            .build()
    }
}

impl Default for RedditFetcherPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for RedditFetcherPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    async fn init(&mut self, _ctx: PluginContext) -> anyhow::Result<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

// Register the plugin and tool
tool!(RedditFetchTool);
plugin!(RedditFetcherPlugin);
