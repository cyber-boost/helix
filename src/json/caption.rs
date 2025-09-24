#![warn(clippy::all, clippy::pedantic)]
use fancy_regex::Regex;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
#[derive(Debug, Clone)]
pub struct E621Config {
    pub filter_tags: bool,
    pub rating_conversions: Option<std::collections::HashMap<String, String>>,
    pub format: Option<String>,
    pub artist_prefix: Option<String>,
    pub artist_suffix: Option<String>,
    pub replace_underscores: bool,
}
impl Default for E621Config {
    fn default() -> Self {
        let mut default_conversions = std::collections::HashMap::new();
        default_conversions.insert("s".to_string(), "safe".to_string());
        default_conversions.insert("q".to_string(), "questionable".to_string());
        default_conversions.insert("e".to_string(), "explicit".to_string());
        Self {
            filter_tags: true,
            rating_conversions: Some(default_conversions),
            format: None,
            artist_prefix: Some("by ".to_string()),
            artist_suffix: None,
            replace_underscores: true,
        }
    }
}
impl E621Config {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    #[must_use]
    pub fn with_filter_tags(mut self, filter_tags: bool) -> Self {
        self.filter_tags = filter_tags;
        self
    }
    #[must_use]
    pub fn with_rating_conversions(
        mut self,
        conversions: Option<std::collections::HashMap<String, String>>,
    ) -> Self {
        self.rating_conversions = conversions;
        self
    }
    #[must_use]
    pub fn with_format(mut self, format: Option<String>) -> Self {
        self.format = format;
        self
    }
    #[must_use]
    pub fn with_artist_prefix(mut self, prefix: Option<String>) -> Self {
        self.artist_prefix = prefix;
        self
    }
    #[must_use]
    pub fn with_artist_suffix(mut self, suffix: Option<String>) -> Self {
        self.artist_suffix = suffix;
        self
    }
    #[must_use]
    pub fn with_replace_underscores(mut self, replace_underscores: bool) -> Self {
        self.replace_underscores = replace_underscores;
        self
    }
    fn get_format(&self) -> &str {
        self.format
            .as_deref()
            .unwrap_or(
                "{rating}, {artists}, {characters}, {species}, {copyright}, {general}, {meta}",
            )
    }
    fn convert_rating(&self, rating: &str) -> String {
        if let Some(conversions) = &self.rating_conversions {
            if let Some(converted) = conversions.get(rating) {
                return converted.clone();
            }
        }
        rating.to_string()
    }
    fn format_artist_name(&self, name: &str) -> String {
        let name = name.replace('_', " ").replace(" (artist)", "");
        let mut formatted = String::new();
        if let Some(prefix) = &self.artist_prefix {
            formatted.push_str(prefix);
        }
        formatted.push_str(&name);
        if let Some(suffix) = &self.artist_suffix {
            formatted.push_str(suffix);
        }
        formatted
    }
}
pub async fn process_file(path: &Path) -> anyhow::Result<()> {
    log::info!("Processing caption file: {}", path.display());
    let content = tokio::fs::read_to_string(path).await?;
    if let Ok(json) = serde_json::from_str::<Value>(&content) {
        log::info!("JSON caption for {}: {:#?}", path.display(), json);
        return Ok(());
    }
    log::info!("Plain text caption for {}: {}", path.display(), content.trim());
    Ok(())
}
pub fn json_to_text(json: &Value) -> anyhow::Result<String> {
    match json {
        Value::String(s) => Ok(s.clone()),
        Value::Object(obj) => {
            if let Some(Value::String(caption)) = obj.get("caption") {
                Ok(caption.clone())
            } else {
                Err(anyhow::anyhow!("No caption field found in JSON object"))
            }
        }
        _ => Err(anyhow::anyhow!("Unsupported JSON format")),
    }
}
pub async fn caption_file_exists_and_not_empty(path: &Path) -> bool {
    if path.exists() {
        match tokio::fs::read_to_string(path).await {
            Ok(content) => !content.trim().is_empty(),
            Err(_) => false,
        }
    } else {
        false
    }
}
pub const IGNORED_E621_TAGS: [&str; 3] = [
    r"^conditional_dnp$",
    r"^\d{4}$",
    r"^\d+:\d+$",
];
#[must_use]
pub fn should_ignore_e621_tag(tag: &str) -> bool {
    IGNORED_E621_TAGS
        .iter()
        .any(|&ignored_tag_pattern| {
            let pattern = Regex::new(ignored_tag_pattern).unwrap();
            pattern.is_match(tag).unwrap_or(false)
        })
}
#[must_use]
pub fn process_e621_tags(tags_dict: &Value, config: Option<&E621Config>) -> Vec<String> {
    let default_config = E621Config::default();
    let config = config.unwrap_or(&default_config);
    let mut processed_tags = Vec::new();
    if let Value::Object(tags) = tags_dict {
        let process_category = |category: &str| {
            tags.get(category)
                .and_then(|t| t.as_array())
                .map(|tags| {
                    tags.iter()
                        .filter_map(|tag| tag.as_str())
                        .filter(|&tag| {
                            !config.filter_tags || !should_ignore_e621_tag(tag)
                        })
                        .map(|tag| {
                            if category == "artist" {
                                config.format_artist_name(tag)
                            } else if config.replace_underscores {
                                tag.replace('_', " ")
                            } else {
                                tag.to_string()
                            }
                        })
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default()
        };
        let categories = [
            "artist",
            "character",
            "species",
            "copyright",
            "general",
            "meta",
        ];
        for category in categories {
            let tags = process_category(category);
            processed_tags.extend(tags);
        }
    }
    processed_tags
}
pub async fn process_e621_json_data(
    data: &Value,
    file_path: &Arc<PathBuf>,
    config: Option<E621Config>,
) -> anyhow::Result<()> {
    let config = config.unwrap_or_default();
    if let Some(post) = data.get("post") {
        if let Some(file_data) = post.get("file") {
            if let Some(url) = file_data.get("url").and_then(|u| u.as_str()) {
                use std::path::Path;
                let filename = Path::new(url).file_stem().unwrap().to_str().unwrap();
                let caption_path = file_path.with_file_name(format!("{filename}.txt"));
                let rating = post.get("rating").and_then(|r| r.as_str()).unwrap_or("q");
                let rating = config.convert_rating(rating);
                let mut tag_groups = std::collections::HashMap::new();
                tag_groups.insert("rating", rating);
                if let Some(Value::Object(tags)) = post.get("tags") {
                    let process_category = |category: &str| {
                        tags.get(category)
                            .and_then(|t| t.as_array())
                            .map(|tags| {
                                tags.iter()
                                    .filter_map(|tag| tag.as_str())
                                    .filter(|&tag| {
                                        !config.filter_tags || !should_ignore_e621_tag(tag)
                                    })
                                    .map(|tag| {
                                        let tag = if config.replace_underscores {
                                            tag.replace('_', " ")
                                        } else {
                                            tag.to_string()
                                        };
                                        if category == "artist" {
                                            config.format_artist_name(&tag)
                                        } else {
                                            tag
                                        }
                                    })
                                    .collect::<Vec<String>>()
                            })
                            .unwrap_or_default()
                    };
                    let artists = process_category("artist");
                    let characters = process_category("character");
                    let species = process_category("species");
                    let copyright = process_category("copyright");
                    let general = process_category("general");
                    let meta = process_category("meta");
                    if !artists.is_empty() {
                        tag_groups.insert("artists", artists.join(", "));
                    }
                    if !characters.is_empty() {
                        tag_groups.insert("characters", characters.join(", "));
                    }
                    if !species.is_empty() {
                        tag_groups.insert("species", species.join(", "));
                    }
                    if !copyright.is_empty() {
                        tag_groups.insert("copyright", copyright.join(", "));
                    }
                    if !general.is_empty() {
                        tag_groups.insert("general", general.join(", "));
                    }
                    if !meta.is_empty() {
                        tag_groups.insert("meta", meta.join(", "));
                    }
                    let mut caption_content = config.get_format().to_string();
                    for (key, value) in &tag_groups {
                        caption_content = caption_content
                            .replace(&format!("{{{key}}}"), value);
                    }
                    caption_content = caption_content
                        .replace(", ,", ",")
                        .replace(",,", ",")
                        .replace(" ,", ",")
                        .trim_matches(&[' ', ','][..])
                        .to_string();
                    if !caption_content.trim().is_empty()
                        && (!config.filter_tags || tag_groups.len() > 1)
                    {
                        tokio::fs::write(&caption_path, &caption_content).await?;
                    }
                }
            }
        }
    }
    Ok(())
}
pub fn format_text_content(content: &str) -> anyhow::Result<String> {
    let content = content.trim();
    let content = content.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(content)
}
pub async fn replace_string(
    path: &Path,
    search: &str,
    replace: &str,
) -> anyhow::Result<()> {
    if search.is_empty() {
        return Ok(());
    }
    let content = tokio::fs::read_to_string(path).await?;
    let mut new_content = content.replace(search, replace);
    if replace.is_empty() {
        new_content = format_text_content(&new_content)?;
    }
    if content != new_content {
        tokio::fs::write(path, new_content).await?;
    }
    Ok(())
}
pub async fn replace_special_chars(path: PathBuf) -> anyhow::Result<()> {
    let content = tokio::fs::read_to_string(&path).await?;
    let new_content = content.replace(['"', '"'], "\"");
    if content != new_content {
        tokio::fs::write(&path, new_content).await?;
    }
    Ok(())
}
pub async fn process_e621_json_file(
    file_path: &Path,
    config: Option<E621Config>,
) -> anyhow::Result<()> {
    let content = tokio::fs::read_to_string(file_path).await?;
    let json_data: Value = serde_json::from_str(&content)?;
    process_e621_json_data(&json_data, &Arc::new(file_path.to_path_buf()), config).await
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;
    #[tokio::test]
    async fn test_process_file_plain_text() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "tag1, tag2, tag3., This is a test caption.")?;
        process_file(&file_path).await?;
        Ok(())
    }
    #[tokio::test]
    async fn test_process_file_json() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.json");
        let json = json!({ "caption" : "A test caption", "tags" : ["tag1", "tag2"] });
        fs::write(&file_path, serde_json::to_string_pretty(&json)?)?;
        process_file(&file_path).await?;
        Ok(())
    }
    #[tokio::test]
    async fn test_process_file_invalid_json() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("invalid.json");
        fs::write(&file_path, "{ invalid json }")?;
        process_file(&file_path).await?;
        Ok(())
    }
    #[test]
    fn test_json_to_text_string() -> anyhow::Result<()> {
        let json = json!("Test caption");
        let text = json_to_text(&json)?;
        assert_eq!(text, "Test caption");
        Ok(())
    }
    #[test]
    fn test_json_to_text_object() -> anyhow::Result<()> {
        let json = json!({ "caption" : "Test caption", "other_field" : "ignored" });
        let text = json_to_text(&json)?;
        assert_eq!(text, "Test caption");
        Ok(())
    }
    #[test]
    fn test_json_to_text_invalid_object() {
        let json = json!({ "not_caption" : "Test caption" });
        assert!(json_to_text(& json).is_err());
    }
    #[test]
    fn test_json_to_text_unsupported_format() {
        let json = json!(42);
        assert!(json_to_text(& json).is_err());
    }
    #[tokio::test]
    async fn test_caption_file_exists_and_not_empty() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let non_existent = temp_dir.path().join("non_existent.txt");
        assert!(! caption_file_exists_and_not_empty(& non_existent). await);
        let empty_file = temp_dir.path().join("empty.txt");
        fs::write(&empty_file, "")?;
        assert!(! caption_file_exists_and_not_empty(& empty_file). await);
        let whitespace_file = temp_dir.path().join("whitespace.txt");
        fs::write(&whitespace_file, "   \n  \t  ")?;
        assert!(! caption_file_exists_and_not_empty(& whitespace_file). await);
        let content_file = temp_dir.path().join("content.txt");
        fs::write(&content_file, "This is a caption")?;
        assert!(caption_file_exists_and_not_empty(& content_file). await);
        Ok(())
    }
    #[test]
    fn test_e621_config_underscore_replacement() {
        let config = E621Config::new().with_replace_underscores(false);
        let json = json!(
            { "artist" : ["artist_name"], "character" : ["character_name"], "general" :
            ["tag_with_underscore"] }
        );
        let tags = process_e621_tags(&json, Some(&config));
        assert!(
            tags.iter().any(| t | t.contains('_')),
            "Tags should preserve underscores when replace_underscores is false"
        );
        let config = E621Config::new().with_replace_underscores(true);
        let tags = process_e621_tags(&json, Some(&config));
        assert!(
            ! tags.iter().any(| t | t.contains('_')),
            "Tags should not contain underscores when replace_underscores is true"
        );
    }
}