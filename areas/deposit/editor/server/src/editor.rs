use editor_web::{CollectionToken, EditorSettings};

use crate::config::EditorConfig;

impl EditorConfig {
    pub(crate) fn editor_settings(&self, css_href: String) -> EditorSettings {
        EditorSettings {
            css_href,
            data_dir: self.data_dir.clone(),
            collection_token: self
                .collection_token
                .as_ref()
                .map(|secret| CollectionToken::new(secret.expose().to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Secret;

    #[test]
    fn the_collection_token_reaches_the_settings() {
        // `None` refuses every collection-report call, so a configured token that
        // stopped here would silently turn the CI poller's reports into 401s.
        let mut config = EditorConfig::default();
        assert!(config.editor_settings(String::new()).collection_token.is_none());

        config.collection_token = Some(Secret::for_test("a-collection-token"));
        let settings = config.editor_settings("/assets/app.css".to_string());
        assert!(settings.collection_token.is_some());
        assert_eq!(settings.css_href, "/assets/app.css");
    }
}
