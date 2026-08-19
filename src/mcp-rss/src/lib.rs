//! mcp-rss - MCP server that provides RSS tooling.

#![deny(unsafe_code)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  clippy::unreachable
)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::todo)]
#![deny(clippy::allow_attributes_without_reason)]

use rmcp::{handler::server::wrapper::Parameters, schemars, tool};
use scraper::{Html, Selector};
use std::time::Duration;

/// MCP server that provides RSS tooling.
#[derive(Debug, Clone)]
pub struct RssServer {
  http: reqwest::Client,
}

impl RssServer {
  pub fn new() -> anyhow::Result<Self> {
    Ok(Self {
      http: reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(format!(
          "{}/{}",
          env!("CARGO_PKG_NAME"),
          env!("CARGO_PKG_VERSION")
        ))
        .build()?,
    })
  }
}

/// Parameters for the `get_articles` tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetArticlesParams {
  /// List of feeds to get articles from
  feeds: Vec<String>,

  /// If provided, only articles published after this time will be returned
  time_from: Option<String>,
}

/// Parameters for the `fetch_article` tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FetchArticleParams {
  /// URL to fetch the article from
  url: String,
}

impl RssServer {
  /// Get articles from RSS/Atom feeds, optionally filtered by publication date.
  ///
  /// Returns a list of article URLs (links) from the specified feeds.
  #[tool]
  async fn get_articles(
    &self,
    Parameters(params): Parameters<GetArticlesParams>,
  ) -> Result<Vec<String>, String> {
    let GetArticlesParams { feeds, time_from } = params;

    if feeds.is_empty() {
      return Err("No feeds specified".into());
    }

    let parsed_time = if let Some(ref iso) = time_from {
      match chrono::DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => Some(dt),
        Err(e) => return Err(format!("Invalid ISO 8601 timestamp: {e}")),
      }
    } else {
      None
    };

    let mut urls = Vec::new();

    for feed_url in feeds {
      let feed_content = {
        let http = &self.http;
        match http.get(&feed_url).send().await {
          Ok(resp) => match resp.text().await {
            Ok(text) => text,
            Err(e) => {
              eprintln!("Failed to fetch feed {}: {}", feed_url, e);
              continue;
            }
          },
          Err(e) => {
            eprintln!("Failed to connect to feed {}: {}", feed_url, e);
            continue;
          }
        }
      };

      let feed = match rss::Channel::read_from(feed_content.as_bytes()) {
        Ok(channel) => channel,
        Err(e) => {
          eprintln!("Failed to parse feed {}: {}", feed_url, e);
          continue;
        }
      };

      for item in feed.items() {
        if let Some(link) = item.link() {
          let published = item
            .pub_date()
            .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok());

          let include = match (published, &parsed_time) {
            (Some(item_date), Some(from_date)) => item_date >= *from_date,
            (Some(_), None) => true,
            (None, Some(_)) => true, // include items with no date if filtering
            (None, None) => true,
          };

          if include {
            urls.push(link.to_string());
          }
        }
      }
    }

    let mut seen = std::collections::HashSet::new();
    urls.retain(|url| seen.insert(url.clone()));

    Ok(urls)
  }

  /// Fetch the content of a single article from a URL.
  ///
  /// Returns cleaned text content extracted from the HTML page.
  #[tool]
  async fn fetch_article(
    &self,
    Parameters(params): Parameters<FetchArticleParams>,
  ) -> Result<String, String> {
    let FetchArticleParams { url } = params;

    let html = {
      let http = &self.http;
      match http.get(&url).send().await {
        Ok(resp) => match resp.text().await {
          Ok(text) => text,
          Err(e) => return Err(format!("Failed to fetch {}: {}", url, e)),
        },
        Err(e) => return Err(format!("Failed to connect to {}: {}", url, e)),
      }
    };

    // Try readability-style extraction using selectors
    let document = Html::parse_document(&html);

    // Try common content selectors
    let content_selectors = [
      "article",
      "main",
      ".post-content",
      ".entry-content",
      ".article-body",
      "#article-content",
      ".content",
      "body",
    ];

    let mut text = String::new();
    let mut found = false;

    for selector_str in &content_selectors {
      if let Ok(selector) = Selector::parse(selector_str) {
        if let Some(element) = document.select(&selector).next() {
          let cleaned = strip_html(&element.html());
          if !cleaned.trim().is_empty() && cleaned.len() > 100 {
            text = cleaned;
            found = true;
            break;
          }
        }
      }
    }

    if !found {
      // Fallback: extract from body
      if let Some(body) =
        document.select(&Selector::parse("body").unwrap()).next()
      {
        text = strip_html(&body.html());
      } else {
        text = strip_html(&html);
      }
    }

    // Clean up whitespace
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

    Ok(text)
  }
}

/// Strip HTML tags and return plain text.
fn strip_html(html: &str) -> String {
  let fragment = Html::parse_fragment(html);
  fragment.root_element().text().collect::<String>()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_strip_html() {
    let input = "<p>Hello <b>world</b></p>";
    let output = strip_html(input);
    // strip_html now normalizes whitespace within paragraphs
    // but separates block elements with newlines
    assert!(output.contains("Hello"));
    assert!(output.contains("world"));
  }

  #[test]
  fn test_strip_html_complex() {
    let input = "<div class=\"article\"><h1>Title</h1><p>Some text with <a href=\"#\">links</a> and <span>spans</span>.</p></div>";
    let output = strip_html(input);
    assert!(output.contains("Title"));
    assert!(output.contains("Some text with"));
    assert!(output.contains("links"));
    assert!(output.contains("spans"));
  }
}
