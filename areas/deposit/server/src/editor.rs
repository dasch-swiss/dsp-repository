use editor_web::EditorSettings;

use crate::config::EditorConfig;

impl EditorConfig {
    pub(crate) fn editor_settings(&self, css_href: String) -> EditorSettings {
        EditorSettings { css_href, data_dir: self.data_dir.clone() }
    }
}
