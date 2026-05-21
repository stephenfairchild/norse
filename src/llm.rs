use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct LlmClient {
    client: Client,
    auth_token: String,
    base_url: String,
    model: String,
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<Msg<'a>>,
}

#[derive(Serialize)]
struct Msg<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct Response {
    content: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

impl LlmClient {
    pub fn from_claude_settings(model_override: Option<String>) -> Result<Self> {
        let home = std::env::var("HOME")?;
        let raw = std::fs::read_to_string(format!("{}/.claude/settings.json", home))?;
        let json: serde_json::Value = serde_json::from_str(&raw)?;
        let env = json["env"].as_object()
            .ok_or_else(|| anyhow::anyhow!("no env in settings.json"))?;

        let get = |k: &str| -> Result<String> {
            env.get(k)
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| anyhow::anyhow!("{} not found in settings.json", k))
        };

        let model = model_override.unwrap_or_else(|| get("ANTHROPIC_MODEL").unwrap_or_default());
        let client = Client::builder().user_agent("norse/0.1").build()?;
        Ok(Self {
            client,
            auth_token: get("ANTHROPIC_AUTH_TOKEN")?,
            base_url: get("ANTHROPIC_BASE_URL")?,
            model,
        })
    }

    pub fn active_model(&self) -> &str {
        &self.model
    }

    pub async fn summarize_diff(&self, diff: &str) -> Result<String> {
        let content = if diff.len() > 8000 { &diff[..8000] } else { diff };
        let prompt = format!(
            r#"Summarize this git diff. Output exactly two sections, no other text:

## Summary
2-4 bullet points, high-level only.
- For UI changes: list affected routes (e.g. /dashboard, /settings/:id).
- For API changes: you MUST list every changed endpoint with its full HTTP method and path, and describe any request/response payload changes (added/removed/renamed fields, type changes, new required fields). Be explicit — do not summarize vaguely.
- For other changes: one short phrase describing what changed.

## Usage
One sentence describing what the new thing does, followed by a minimal code example showing how to use it.
- Any API change (new, modified, or removed endpoint) → you MUST show a curl example. Include the full route, all required headers, and a realistic request body with every field. If the response shape changed, show the new response JSON as a comment below the curl. No exceptions.
- New function/method → call-site code example in the repo's language
- Config change → example config snippet
- UI route → example URL or component usage
- No clear new usage surface → omit the code block and write "No direct usage change."

Diff:
{}"#,
            content
        );

        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let resp: Response = self.client
            .post(&url)
            .header("x-api-key", &self.auth_token)
            .header("anthropic-version", "2023-06-01")
            .json(&Request {
                model: &self.model,
                max_tokens: 512,
                messages: vec![Msg { role: "user", content: &prompt }],
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.content.into_iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join(""))
    }

    pub async fn ask_repo(&self, repo: &str, question: &str, history: &[(String, String)], progress: tokio::sync::mpsc::Sender<String>) -> Result<String> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let repo_url = format!("https://github.com/{}", repo);

        let tool = json!({
            "name": "github_cli",
            "description": "Run a read-only gh CLI command to fetch information from a GitHub repository. Use this to look up READMEs, file contents, issues, PRs, releases, and other repo data.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "A read-only gh CLI command, e.g. 'gh api repos/owner/repo/readme --jq .content | base64 -d' or 'gh api repos/owner/repo/contents/src/main.rs --jq .content | base64 -d'"
                    }
                },
                "required": ["command"]
            }
        });

        let mut messages: Vec<serde_json::Value> = history.iter().flat_map(|(q, a)| [
            json!({"role": "user", "content": q.as_str()}),
            json!({"role": "assistant", "content": [{"type": "text", "text": a.as_str()}]}),
        ]).collect();
        messages.push(json!({"role": "user", "content": question}));

        for _ in 0..20 {
            let body = json!({
                "model": self.model,
                "max_tokens": 1024,
                "system": format!("You are a code assistant for the GitHub repo {}. Use the github_cli tool to read from the repo and answer the question. Only use read-only commands.", repo_url),
                "tools": [tool],
                "messages": messages,
            });

            let resp: serde_json::Value = self.client
                .post(&url)
                .header("x-api-key", &self.auth_token)
                .header("anthropic-version", "2023-06-01")
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            let stop_reason = resp["stop_reason"].as_str().unwrap_or("").to_string();
            let content = resp["content"].as_array().cloned().unwrap_or_default();

            messages.push(json!({"role": "assistant", "content": content}));

            if stop_reason == "tool_use" {
                let mut tool_results: Vec<serde_json::Value> = Vec::new();
                for block in &content {
                    if block["type"].as_str() == Some("tool_use") {
                        let id = block["id"].as_str().unwrap_or("").to_string();
                        let command = block["input"]["command"].as_str().unwrap_or("").to_string();
                        let _ = progress.send(format!("$ {}", command)).await;
                        let output = run_gh_command(&command).await;
                        let _ = progress.send(format!("  {} bytes", output.len())).await;
                        tool_results.push(json!({
                            "type": "tool_result",
                            "tool_use_id": id,
                            "content": output,
                        }));
                    }
                }
                messages.push(json!({"role": "user", "content": tool_results}));
            } else {
                let text: String = content.iter()
                    .filter(|b| b["type"].as_str() == Some("text"))
                    .filter_map(|b| b["text"].as_str().map(String::from))
                    .collect::<Vec<_>>()
                    .join("");
                return Ok(text);
            }
        }

        Ok("Reached tool call limit without a final answer.".into())
    }

    pub async fn ask_repos(&self, repos: &[String], question: &str, history: &[(String, String)], progress: tokio::sync::mpsc::Sender<String>) -> Result<String> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let repo_list = repos.iter()
            .map(|r| format!("  - https://github.com/{}", r))
            .collect::<Vec<_>>()
            .join("\n");

        let tool = json!({
            "name": "github_cli",
            "description": "Run a read-only gh CLI command to fetch information from GitHub repositories.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "A read-only gh CLI command"
                    }
                },
                "required": ["command"]
            }
        });

        let system = format!(
            "You are a code assistant with access to these GitHub repositories:\n{}\nUse the github_cli tool to read from any of these repos and answer the question. Only use read-only commands.",
            repo_list
        );

        let mut messages: Vec<serde_json::Value> = history.iter().flat_map(|(q, a)| [
            json!({"role": "user", "content": q.as_str()}),
            json!({"role": "assistant", "content": [{"type": "text", "text": a.as_str()}]}),
        ]).collect();
        messages.push(json!({"role": "user", "content": question}));

        for _ in 0..20 {
            let body = json!({
                "model": self.model,
                "max_tokens": 1024,
                "system": system,
                "tools": [tool],
                "messages": messages,
            });

            let resp: serde_json::Value = self.client
                .post(&url)
                .header("x-api-key", &self.auth_token)
                .header("anthropic-version", "2023-06-01")
                .json(&body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            let stop_reason = resp["stop_reason"].as_str().unwrap_or("").to_string();
            let content = resp["content"].as_array().cloned().unwrap_or_default();

            messages.push(json!({"role": "assistant", "content": content}));

            if stop_reason == "tool_use" {
                let mut tool_results: Vec<serde_json::Value> = Vec::new();
                for block in &content {
                    if block["type"].as_str() == Some("tool_use") {
                        let id = block["id"].as_str().unwrap_or("").to_string();
                        let command = block["input"]["command"].as_str().unwrap_or("").to_string();
                        let _ = progress.send(format!("$ {}", command)).await;
                        let output = run_gh_command(&command).await;
                        let _ = progress.send(format!("  {} bytes", output.len())).await;
                        tool_results.push(json!({
                            "type": "tool_result",
                            "tool_use_id": id,
                            "content": output,
                        }));
                    }
                }
                messages.push(json!({"role": "user", "content": tool_results}));
            } else {
                let text: String = content.iter()
                    .filter(|b| b["type"].as_str() == Some("text"))
                    .filter_map(|b| b["text"].as_str().map(String::from))
                    .collect::<Vec<_>>()
                    .join("");
                return Ok(text);
            }
        }

        Ok("Reached tool call limit without a final answer.".into())
    }

    pub async fn generate_news_summary(&self, activity_text: &str) -> Result<String> {
        let content = if activity_text.len() > 12000 { &activity_text[..12000] } else { activity_text };
        let prompt = format!(
            r#"You are summarizing 24 hours of GitHub activity across repositories I watch.

Instructions:
1. DISCARD any PR that is purely: dependency upgrades, version bumps, chore/cleanup, linting, test-only, docs, or CI config with no user-facing impact. Be aggressive about discarding noise.
2. For the remaining PRs, write one paragraph each describing what the change actually does and why it matters to the business or users. Be direct and specific — no workflow language ("a PR was opened"), no meta-commentary.
3. Order by business impact, highest first. Max 20 items total.
4. Separate each paragraph with exactly "---" on its own line.
5. End each paragraph with the GitHub PR URL on its own line, and the Jira ticket (if present) on its own line.
6. No bullet points. No headers. Just the paragraphs.

GitHub activity (last 24 hours):
{}"#,
            content
        );

        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let resp: Response = self.client
            .post(&url)
            .header("x-api-key", &self.auth_token)
            .header("anthropic-version", "2023-06-01")
            .json(&Request {
                model: &self.model,
                max_tokens: 4096,
                messages: vec![Msg { role: "user", content: &prompt }],
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.content.into_iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join(""))
    }

    pub async fn ask_diff(&self, diff: &str, repo: &str, question: &str) -> Result<String> {
        let content = if diff.len() > 8000 { &diff[..8000] } else { diff };
        let repo_url = format!("https://github.com/{}", repo);
        let prompt = format!(
            "You are a code review assistant. Repo: {}\n\nDiff:\n{}\n\nQuestion: {}\n\nAnswer briefly and directly.",
            repo_url, content, question
        );

        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let resp: Response = self.client
            .post(&url)
            .header("x-api-key", &self.auth_token)
            .header("anthropic-version", "2023-06-01")
            .json(&Request {
                model: &self.model,
                max_tokens: 512,
                messages: vec![Msg { role: "user", content: &prompt }],
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.content.into_iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join(""))
    }
}

async fn run_gh_command(command: &str) -> String {
    let cmd = command.trim();
    if !is_safe_gh_command(cmd) {
        return "Error: only read-only gh commands are permitted.".to_string();
    }
    match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            // Limit output to 4000 chars so it fits back into context
            let out = if stdout.is_empty() { stderr } else { stdout };
            if out.len() > 4000 { out[..4000].to_string() } else { out }
        }
        Err(e) => format!("Error: {}", e),
    }
}

fn is_safe_gh_command(cmd: &str) -> bool {
    if !cmd.starts_with("gh ") {
        return false;
    }
    // Block any write operations
    let blocked = [
        "--method POST", "--method PUT", "--method DELETE", "--method PATCH",
        " create", " delete", " close", " merge", " edit", " add", " remove",
        " update", " set", " enable", " disable", " deploy",
    ];
    !blocked.iter().any(|b| cmd.contains(b))
}
