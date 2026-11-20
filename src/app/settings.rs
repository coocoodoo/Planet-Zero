use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsSection {
    General,
    Advanced,
}

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
        self.settings_section = None;
        self.selection = 0;
    }
    pub(super) fn close_settings(&mut self) {
        (self.page, self.selection) = self.settings_return;
    }
    pub(crate) fn settings_rows(&self) -> usize {
        if self.settings_section.is_some() {
            5
        } else {
            3
        }
    }
    pub(super) fn settings_back(&mut self) {
        if let Some(section) = self.settings_section.take() {
            self.selection = usize::from(section == SettingsSection::Advanced);
        } else {
            self.close_settings();
        }
    }
    pub(super) fn activate_setting(&mut self) {
        if self.selection == self.settings_rows() - 1 {
            self.settings_back();
        } else if self.settings_section.is_none() {
            self.settings_section = Some(if self.selection == 0 {
                SettingsSection::General
            } else {
                SettingsSection::Advanced
            });
            self.selection = 0;
        } else {
            self.adjust_setting(1);
        }
    }
    pub(super) fn adjust_setting(&mut self, direction: i32) {
        match (self.settings_section, self.selection) {
            (Some(SettingsSection::General), 0) => self.flight.hud = !self.flight.hud,
            (Some(SettingsSection::General), 1) => self.portal_labels = !self.portal_labels,
            (Some(SettingsSection::General), 2) => {
                self.audio_settings.profile =
                    (self.audio_settings.profile as i32 + direction).rem_euclid(4) as u8
            }
            (Some(SettingsSection::General), 3) => {
                self.audio_settings.volume =
                    (self.audio_settings.volume as i32 + direction).clamp(0, 10) as u8
            }
            (Some(SettingsSection::Advanced), 0) => self.flight.faceted = !self.flight.faceted,
            (Some(SettingsSection::Advanced), 1) => {
                self.flight.dynamic_clouds = !self.flight.dynamic_clouds
            }
            (Some(SettingsSection::Advanced), 2) => {
                self.flight.dynamic_lighting = !self.flight.dynamic_lighting
            }
            (Some(SettingsSection::Advanced), 3) => {
                self.flight.portal_preview = !self.flight.portal_preview
            }
            _ => return,
        }
        if (self.settings_section == Some(SettingsSection::Advanced) || self.selection == 1)
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
