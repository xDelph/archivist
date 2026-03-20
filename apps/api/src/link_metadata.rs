use crate::{AppState, auth::SessionClaims};
use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use html_escape::decode_html_entities;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct LinkMetadataQuery {
    url: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct LinkMetadataResponse {
    url: String,
    title: Option<String>,
    description: Option<String>,
    site_name: Option<String>,
    image: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ErrorResponse {
    error: &'static str,
}

pub(crate) async fn link_metadata(
    State(_state): State<AppState>,
    Extension(_claims): Extension<SessionClaims>,
    Query(query): Query<LinkMetadataQuery>,
) -> Result<Json<LinkMetadataResponse>, (StatusCode, Json<ErrorResponse>)> {
    let url = query.url.as_deref().ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "missing_url",
        }),
    ))?;
    let url = parse_metadata_url(url).ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "invalid_url",
        }),
    ))?;

    Ok(Json(fetch_link_metadata(url).await))
}

async fn fetch_link_metadata(url: reqwest::Url) -> LinkMetadataResponse {
    let client = reqwest::Client::builder()
        .user_agent("Arkivist Link Preview")
        .build();
    let Ok(client) = client else {
        return empty_metadata(url);
    };

    let response = client
        .get(url.clone())
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml;q=0.9,*/*;q=0.1",
        )
        .send()
        .await;
    let Ok(response) = response else {
        return empty_metadata(url);
    };

    let resolved_url = response.url().clone();
    let is_html = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_none_or(|value| value.contains("text/html") || value.contains("xhtml"));
    if !is_html {
        return empty_metadata(resolved_url);
    }

    let body = response.text().await;
    let Ok(body) = body else {
        return empty_metadata(resolved_url);
    };

    parse_link_metadata(&resolved_url, &body)
}

fn parse_link_metadata(url: &reqwest::Url, html: &str) -> LinkMetadataResponse {
    let title = meta_content(html, "og:title")
        .or_else(|| meta_content(html, "twitter:title"))
        .or_else(|| title_tag(html));
    let description = meta_content(html, "og:description")
        .or_else(|| meta_content(html, "twitter:description"))
        .or_else(|| meta_content(html, "description"));
    let site_name = meta_content(html, "og:site_name");
    let image = meta_content(html, "og:image")
        .or_else(|| meta_content(html, "twitter:image"))
        .and_then(|value| resolve_url(url, &value));

    LinkMetadataResponse {
        url: url.as_str().to_owned(),
        title,
        description,
        site_name,
        image,
    }
}

fn meta_content(html: &str, key: &str) -> Option<String> {
    let pattern = Regex::new(r"(?is)<meta\b[^>]*>").expect("valid meta tag regex");
    for tag in pattern.find_iter(html) {
        let tag = tag.as_str();
        let property = attribute_value(tag, "property");
        let name = attribute_value(tag, "name");
        if (property
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(key))
            || name
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(key)))
            && let Some(content) = attribute_value(tag, "content")
                .as_deref()
                .and_then(normalize_metadata)
        {
            return Some(content);
        }
    }

    None
}

fn title_tag(html: &str) -> Option<String> {
    let pattern =
        Regex::new(r"(?is)<title\b[^>]*>(?P<title>.*?)</title>").expect("valid title tag regex");
    pattern
        .captures(html)
        .and_then(|captures| captures.name("title"))
        .map(|value| value.as_str())
        .and_then(normalize_metadata)
}

fn attribute_value(tag: &str, attribute: &str) -> Option<String> {
    let pattern = Regex::new(&format!(
        r#"(?is)\b{}\s*=\s*(?:"(?P<double>[^"]*)"|'(?P<single>[^']*)'|(?P<bare>[^\s>]+))"#,
        regex::escape(attribute)
    ))
    .expect("valid attribute regex");
    let captures = pattern.captures(tag)?;
    captures
        .name("double")
        .or_else(|| captures.name("single"))
        .or_else(|| captures.name("bare"))
        .map(|value| value.as_str().to_owned())
}

fn normalize_metadata(value: &str) -> Option<String> {
    let decoded = decode_html_entities(value).into_owned();
    let normalized = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty()).then_some(normalized)
}

fn resolve_url(base_url: &reqwest::Url, value: &str) -> Option<String> {
    reqwest::Url::parse(value)
        .ok()
        .or_else(|| base_url.join(value).ok())
        .map(|url| url.to_string())
}

fn empty_metadata(url: reqwest::Url) -> LinkMetadataResponse {
    LinkMetadataResponse {
        url: url.to_string(),
        title: None,
        description: None,
        site_name: None,
        image: None,
    }
}

fn parse_metadata_url(value: &str) -> Option<reqwest::Url> {
    let url = reqwest::Url::parse(value.trim()).ok()?;
    matches!(url.scheme(), "http" | "https").then_some(url)
}

#[cfg(test)]
#[path = "link_metadata_tests.rs"]
mod tests;
