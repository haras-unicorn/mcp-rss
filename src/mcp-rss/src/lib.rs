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

use rmcp::{
  Json, handler::server::wrapper::Parameters, schemars, tool, tool_router,
};
use scraper::Html;
use std::time::Duration;

#[allow(clippy::unwrap_used, reason = "these are all valid selectors")]
mod selectors {
  use scraper::Selector;

  lazy_static::lazy_static! {
    pub static ref BODY_SELECTOR: Selector = Selector::parse("body").unwrap();

    pub static ref CONTENT_SELECTORS: Vec<Selector> = [
        "article",
        "main",
        ".post-content",
        ".entry-content",
        ".article-body",
        "#article-content",
        ".content",
        "body",
      ]
        .map(|selector| Selector::parse(selector).unwrap())
        .to_vec();
  }
}

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

#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
struct GetArticlesInput {
  /// List of RSS/Atom feed URLs to fetch articles from
  feeds: Vec<String>,
  /// If provided, only articles published after this time will be returned (ISO 8601 format)
  time_from: Option<String>,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct GetArticlesOutput {
  /// List of article URLs
  urls: Vec<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct FetchArticleInput {
  /// URL of the article to fetch
  url: String,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct FetchArticleOutput {
  /// Cleaned text content of the article
  content: String,
}

#[tool_router(server_handler)]
impl RssServer {
  /// Get articles from RSS/Atom feeds, optionally filtered by publication date.
  ///
  /// Returns a list of article URLs (links) from the specified feeds.
  #[tool(name = "get_articles", description = "Get articles from RSS/Atom feeds, optionally filtered by publication date.")]
  async fn get_articles(
    &self,
    Parameters(input): Parameters<GetArticlesInput>,
  ) -> Json<GetArticlesOutput> {
    if input.feeds.is_empty() {
      return Json(GetArticlesOutput { urls: vec![] });
    }

    let parsed_time = if let Some(ref iso) = input.time_from {
      match chrono::DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => Some(dt),
        Err(e) => {
          eprintln!("Invalid ISO 8601 timestamp: {e}");
          None
        }
      }
    } else {
      None
    };

    let mut urls = Vec::new();

    for feed_url in input.feeds {
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
            (None, Some(_)) => true,
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

    Json(GetArticlesOutput { urls })
  }

  /// Fetch the content of a single article from a URL.
  ///
  /// Returns cleaned text content extracted from the HTML page.
  #[tool(name = "fetch_article", description = "Fetch the content of a single article from a URL.")]
  async fn fetch_article(
    &self,
    Parameters(input): Parameters<FetchArticleInput>,
  ) -> Json<FetchArticleOutput> {
    let html = {
      let http = &self.http;
      match http.get(&input.url).send().await {
        Ok(resp) => match resp.text().await {
          Ok(text) => text,
          Err(e) => {
            eprintln!("Failed to fetch {}: {}", input.url, e);
            return Json(FetchArticleOutput { content: format!("Failed to fetch: {e}") });
          }
        },
        Err(e) => {
          eprintln!("Failed to connect to {}: {}", input.url, e);
          return Json(FetchArticleOutput { content: format!("Failed to connect: {e}") });
        }
      }
    };

    // Try readability-style extraction using selectors
    let document = Html::parse_document(&html);

    // Try common content selectors
    let mut text = String::new();
    let mut found = false;

    for selector in selectors::CONTENT_SELECTORS.iter() {
      if let Some(element) = document.select(selector).next() {
        let cleaned = strip_html(&element.html());
        if !cleaned.trim().is_empty() && cleaned.len() > 100 {
          text = cleaned;
          found = true;
          break;
        }
      }
    }

    if !found {
      // Fallback: extract from body
      if let Some(body) = document.select(&selectors::BODY_SELECTOR).next() {
        text = strip_html(&body.html());
      } else {
        text = strip_html(&html);
      }
    }

    // Clean up whitespace
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

    Json(FetchArticleOutput { content: text })
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
  use wiremock::{Mock, MockServer, ResponseTemplate};

  fn make_server() -> RssServer {
    RssServer::new().expect("server construction")
  }

  // --- strip_html (no HTTP needed) ---

  #[test]
  fn test_strip_html_simple() {
    let input = "<p>Hello <b>world</b></p>";
    let output = strip_html(input);
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

  // --- get_articles ---

  #[tokio::test]
  async fn test_get_articles_basic() {
    let server = make_server();
    let mock_server = MockServer::start().await;

    let rss = r#"<?xml version="1.0"?>
    <rss version="2.0">
      <channel>
        <title>Test</title>
        <item><title>One</title><link>https://example.com/one</link></item>
        <item><title>Two</title><link>https://example.com/two</link></item>
      </channel>
    </rss>"#;

    Mock::given(wiremock::matchers::path("/feed.xml"))
      .respond_with(ResponseTemplate::new(200).set_body_string(rss))
      .mount(&mock_server)
      .await;

    let input = GetArticlesInput {
      feeds: vec![format!("{}/feed.xml", mock_server.uri())],
      time_from: None,
    };
    let result = server.get_articles(Parameters(input)).await;
    let urls = result.urls;

    assert_eq!(urls.len(), 2);
    assert!(urls.contains(&"https://example.com/one".to_string()));
    assert!(urls.contains(&"https://example.com/two".to_string()));
  }

  #[tokio::test]
  async fn test_get_articles_time_filter() {
    let server = make_server();
    let mock_server = MockServer::start().await;

    let rss = r#"<?xml version="1.0"?>
    <rss version="2.0">
      <channel>
        <title>Test</title>
        <item><title>Old</title><link>https://example.com/old</link><pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate></item>
        <item><title>Recent</title><link>https://example.com/recent</link><pubDate>Fri, 19 Jul 2026 12:00:00 +0000</pubDate></item>
        <item><title>No Date</title><link>https://example.com/nodeate</link></item>
      </channel>
    </rss>"#;

    Mock::given(wiremock::matchers::path("/feed.xml"))
      .respond_with(ResponseTemplate::new(200).set_body_string(rss))
      .mount(&mock_server)
      .await;

    let input = GetArticlesInput {
      feeds: vec![format!("{}/feed.xml", mock_server.uri())],
      time_from: Some("2026-01-01T00:00:00+00:00".to_string()),
    };
    let result = server.get_articles(Parameters(input)).await;
    let urls = result.urls;

    // "Old" is before the filter date — excluded
    assert!(!urls.contains(&"https://example.com/old".to_string()));
    // "Recent" is after the filter date — included
    assert!(urls.contains(&"https://example.com/recent".to_string()));
    // "No Date" has no pubDate — included regardless of filter
    assert!(urls.contains(&"https://example.com/nodeate".to_string()));
  }

  #[tokio::test]
  async fn test_get_articles_dedup() {
    let server = make_server();
    let mock_server = MockServer::start().await;

    let rss = r#"<?xml version="1.0"?>
    <rss version="2.0">
      <channel>
        <title>Test</title>
        <item><title>A</title><link>https://example.com/a</link></item>
        <item><title>B</title><link>https://example.com/b</link></item>
      </channel>
    </rss>"#;

    Mock::given(wiremock::matchers::path("/feed1.xml"))
      .respond_with(ResponseTemplate::new(200).set_body_string(rss))
      .mount(&mock_server)
      .await;

    Mock::given(wiremock::matchers::path("/feed2.xml"))
      .respond_with(ResponseTemplate::new(200).set_body_string(rss))
      .mount(&mock_server)
      .await;

    let input = GetArticlesInput {
      feeds: vec![
        format!("{}/feed1.xml", mock_server.uri()),
        format!("{}/feed2.xml", mock_server.uri()),
      ],
      time_from: None,
    };
    let result = server.get_articles(Parameters(input)).await;
    let urls = result.urls;

    // Each feed has 2 items, but they're the same URLs — should deduplicate to 2
    assert_eq!(urls.len(), 2);
  }

  #[tokio::test]
  async fn test_get_articles_invalid_xml() {
    let server = make_server();
    let mock_server = MockServer::start().await;

    Mock::given(wiremock::matchers::path("/bad.xml"))
      .respond_with(ResponseTemplate::new(200).set_body_string("not xml at all {{{"))
      .mount(&mock_server)
      .await;

    let input = GetArticlesInput {
      feeds: vec![format!("{}/bad.xml", mock_server.uri())],
      time_from: None,
    };
    let result = server.get_articles(Parameters(input)).await;
    assert!(result.urls.is_empty());
  }

  #[tokio::test]
  async fn test_get_articles_empty_feeds() {
    let server = make_server();
    let input = GetArticlesInput {
      feeds: vec![],
      time_from: None,
    };
    let result = server.get_articles(Parameters(input)).await;
    assert!(result.urls.is_empty());
  }

  // --- fetch_article ---

  #[tokio::test]
  async fn test_fetch_article_article_selector() {
    let server = make_server();
    let mock_server = MockServer::start().await;

    let html = r#"<html><body>
      <nav>Navigation should be ignored</nav>
      <article><h1>Article Title</h1><p>This is the main article content that should be extracted.</p></article>
      <footer>Footer should be ignored</footer>
    </body></html>"#;

    Mock::given(wiremock::matchers::path("/article.html"))
      .respond_with(ResponseTemplate::new(200).set_body_string(html))
      .mount(&mock_server)
      .await;

    let input = FetchArticleInput {
      url: format!("{}/article.html", mock_server.uri()),
    };
    let result = server.fetch_article(Parameters(input)).await;

    assert!(result.content.contains("Article Title"));
    assert!(result.content.contains("main article content"));
    assert!(!result.content.contains("Navigation"));
    assert!(!result.content.contains("Footer"));
  }

  #[tokio::test]
  async fn test_fetch_article_class_selector() {
    let server = make_server();
    let mock_server = MockServer::start().await;

    let html = r#"<html><body>
      <header>Header content</header>
      <div class="entry-content"><h1>Blog Post</h1><p>Blog post body text here.</p></div>
    </body></html>"#;

    Mock::given(wiremock::matchers::path("/blog.html"))
      .respond_with(ResponseTemplate::new(200).set_body_string(html))
      .mount(&mock_server)
      .await;

    let input = FetchArticleInput {
      url: format!("{}/blog.html", mock_server.uri()),
    };
    let result = server.fetch_article(Parameters(input)).await;

    assert!(result.content.contains("Blog Post"));
    assert!(result.content.contains("Blog post body text"));
    assert!(!result.content.contains("Header content"));
  }

  #[tokio::test]
  async fn test_fetch_article_fallback() {
    let server = make_server();
    let mock_server = MockServer::start().await;

    // No article, main, or recognized class — should fall back to body
    let html = r#"<html><body>
      <div><p>This is all there is. Just some plain content.</p></div>
    </body></html>"#;

    Mock::given(wiremock::matchers::path("/plain.html"))
      .respond_with(ResponseTemplate::new(200).set_body_string(html))
      .mount(&mock_server)
      .await;

    let input = FetchArticleInput {
      url: format!("{}/plain.html", mock_server.uri()),
    };
    let result = server.fetch_article(Parameters(input)).await;

    assert!(result.content.contains("all there is"));
    assert!(result.content.contains("plain content"));
  }

  #[tokio::test]
  async fn test_fetch_article_selectors_prioritized() {
    let server = make_server();
    let mock_server = MockServer::start().await;

    // Both <article> and .entry-content present — should pick <article> first
    let html = r#"<!DOCTYPE html>
    <html>
    <body>
      <article><h1>Article Content</h1></article>
      <div class="entry-content">Entry Content</div>
    </body>
    </html>"#;

    Mock::given(wiremock::matchers::path("/multi.html"))
      .respond_with(ResponseTemplate::new(200).set_body_string(html))
      .mount(&mock_server)
      .await;

    let input = FetchArticleInput {
      url: format!("{}/multi.html", mock_server.uri()),
    };
    let result = server.fetch_article(Parameters(input)).await;

    assert!(result.content.contains("Article Content"));
    // Should NOT contain the .entry-content text since article was matched first
    assert!(!result.content.contains("Entry Content"));
  }
}
