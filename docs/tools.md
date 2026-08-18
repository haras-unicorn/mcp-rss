# Tools

The `mcp-rss` server exposes the following tools.

## `get_articles`

Fetches article URLs from RSS/Atom feeds.

- **Parameters**:
  - `feeds` (`Vec<String>`): List of feed URLs to fetch.
  - `time_from` (`Option<String>`): Optional ISO 8601 timestamp. Only articles published after this time are returned.
- **Returns**: A deduplicated list of article URLs (`Vec<String>`).
- **Notes**: If an article lacks a publication date, it is included regardless of `time_from` filtering.

## `fetch_article`

Fetches and cleans the text content of a single article from a URL.

- **Parameters**:
  - `url` (`String`): The URL of the article to fetch.
- **Returns**: Cleaned text content (`String`).
- **Notes**: Uses a cascade of CSS selectors (`article`, `main`, `.post-content`, etc.) to extract the main content. Falls back to the full `<body>` if no specific content block is found.

