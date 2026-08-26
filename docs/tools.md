# Tools

The `mcp-rss` server exposes the following tools.

## `get_articles`

Fetches articles from RSS/Atom feeds.

- **Parameters**:
  - `feeds` (`Vec<String>`): List of feed URLs to fetch.
  - `time_from` (`Option<String>`): Optional ISO 8601 timestamp. Only articles
    published after this time are returned.
- **Returns**: A list of `Article` objects, deduplicated by URL. Each article
  has:
  - `link` (`String`): Canonical URL of the article.
  - `title` (`String`): Article title (empty string if missing).
  - `id` (`String`): Feed-specific identifier (GUID).
  - `published` (`Option<String>`): Publication date in RFC 3333 format, or
    `null`.
  - `description` (`Option<String>`): Feed-specific summary/description, or
    `null`.
- **Notes**: If an article lacks a publication date, it is included regardless
  of `time_from` filtering. Articles are deduplicated by link across all feeds.
  `get_articles` is never subject to `MCP_RSS_ALLOWED_DOMAINS` — feeds may be
  fetched from any host.

## `fetch_article`

Fetches and cleans the text content of a single article from a URL.

- **Parameters**:
  - `url` (`String`): The URL of the article to fetch.
- **Returns**: Cleaned text content (`String`).
- **Notes**: Uses a cascade of CSS selectors (`article`, `main`,
  `.post-content`, etc.) to extract the main content. Falls back to the full
  `<body>` if no specific content block is found.
- - **Domain restriction**: If the `MCP_RSS_ALLOWED_DOMAINS` environment
    variable is set to a comma-separated list of domains, `fetch_article` only
    accepts URLs whose hostname is in that list. An entry matches its own host
    and any of its subdomains (e.g. `index.hr` also allows `www.index.hr`). The
    comparison is case-insensitive and surrounding whitespace around each entry
    is ignored. When unset, any URL may be fetched. See
    [Restricting article fetching](./introduction.md#restricting-article-fetching).
