use super::*;

#[derive(serde::Deserialize)]
#[serde(default)]
pub(super) struct Settings {
    pub faceted: bool,
    pub portal_labels: bool,
    pub dynamic_clouds: bool,
    pub dynamic_lighting: bool,
    pub portal_preview: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            faceted: true,
            portal_labels: true,
            dynamic_clouds: true,
            dynamic_lighting: true,
            portal_preview: true,
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
        if self.selection == 0 {
            self.settings_advanced = !self.settings_advanced;
            return;
        }
        if self.selection == 5 {
            self.close_settings();
            return;
        }
        if self.settings_advanced {
            match self.selection {
                1 => self.flight.faceted = !self.flight.faceted,
                2 => self.flight.dynamic_clouds = !self.flight.dynamic_clouds,
                3 => self.flight.dynamic_lighting = !self.flight.dynamic_lighting,
                4 => self.flight.portal_preview = !self.flight.portal_preview,
                _ => return,
            }
        } else {
            match self.selection {
                1 => self.flight.hud = !self.flight.hud,
                2 => self.portal_labels = !self.portal_labels,
                3 => {
                    self.audio_settings.profile =
                        (self.audio_settings.profile as i32 + direction).rem_euclid(4) as u8
                }
                4 => {
                    self.audio_settings.volume =
                        (self.audio_settings.volume as i32 + direction).clamp(0, 10) as u8
                }
                _ => return,
            }
        }
        if (self.settings_advanced || self.selection == 2)
            && let Some(path) = &self.settings_path
        {
            let result = (|| -> std::io::Result<()> {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(
                    path,
                    format!(
                        "faceted = {}\nportal_labels = {}\ndynamic_clouds = {}\ndynamic_lighting = {}\nportal_preview = {}\n",
                        self.flight.faceted,
                        self.portal_labels,
                        self.flight.dynamic_clouds,
                        self.flight.dynamic_lighting,
                        self.flight.portal_preview
                    ),
                )
            })();
            if let Err(error) = result {
                eprintln!("保存显示设置失败：{error}");
            }
        }
    }
}
