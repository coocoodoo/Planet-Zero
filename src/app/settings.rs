use super::*;

#[derive(serde::Deserialize)]
#[serde(default)]
pub(super) struct Settings {
    pub faceted: bool,
    pub portal_labels: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            faceted: true,
            portal_labels: true,
        }
    }
}
impl Settings {
    pub fn load(path: Option<&std::path::Path>) -> Self {
        path.and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }
}
impl App {
    pub(super) fn open_settings(&mut self) {
        self.settings_return = (self.page, self.selection);
        self.page = Page::Settings;
        self.selection = 0;
    }
    pub(super) fn close_settings(&mut self) {
        (self.page, self.selection) = self.settings_return;
    }
    pub(super) fn adjust_setting(&mut self, direction: i32) {
        match self.selection {
            0 => self.flight.faceted = !self.flight.faceted,
            1 => self.portal_labels = !self.portal_labels,
            2 => self.flight.hud = !self.flight.hud,
            3 => {
                self.audio_settings.profile =
                    (self.audio_settings.profile as i32 + direction).rem_euclid(4) as u8
            }
            4 => {
                self.audio_settings.volume =
                    (self.audio_settings.volume as i32 + direction).clamp(0, 10) as u8
            }
            _ => {
                self.close_settings();
                return;
            }
        }
        if self.selection <= 1
            && let Some(path) = &self.settings_path
        {
            let result = (|| -> std::io::Result<()> {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(
                    path,
                    format!(
                        "faceted = {}\nportal_labels = {}\n",
                        self.flight.faceted, self.portal_labels
                    ),
                )
            })();
            if let Err(error) = result {
                eprintln!("保存显示设置失败：{error}");
            }
        }
    }
}
