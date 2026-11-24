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
    pub cloud_pixel_texture: bool,
    pub dynamic_lighting: bool,
    pub smooth_sky_gradient: bool,
    pub portal_preview: bool,
    pub hud: bool,
    pub volume: u8,
    pub sound_profile: u8,
    pub aim_assist: bool,
    pub tilt_aim: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            faceted: true,
            portal_labels: true,
            dynamic_clouds: true,
            cloud_pixel_texture: true,
            dynamic_lighting: true,
            smooth_sky_gradient: true,
            portal_preview: true,
            hud: true,
            volume: crate::audio::Settings::default().volume,
            sound_profile: 0,
            aim_assist: true,
            tilt_aim: true,
        }
    }
}
impl Settings {
    pub fn load(path: Option<&std::path::Path>) -> Self {
        path.and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }
    pub fn audio(&self) -> crate::audio::Settings {
        crate::audio::Settings {
            profile: self.sound_profile % 4,
            volume: self.volume.min(10),
        }
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
        match self.settings_section {
            Some(SettingsSection::General) => 7,
            Some(SettingsSection::Advanced) => 7,
            None => 3,
        }
    }
    pub(crate) fn settings_row_height(&self) -> usize {
        match self.settings_section {
            Some(SettingsSection::General) => 11,
            Some(SettingsSection::Advanced) => 11,
            None => 28,
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
            (Some(SettingsSection::General), 4) => self.aim_assist = !self.aim_assist,
            (Some(SettingsSection::General), 5) => {
                self.tilt_aim = !self.tilt_aim;
                self.sky_recenter();
            }
            (Some(SettingsSection::Advanced), 0) => self.flight.faceted = !self.flight.faceted,
            (Some(SettingsSection::Advanced), 1) => {
                self.flight.dynamic_clouds = !self.flight.dynamic_clouds
            }
            (Some(SettingsSection::Advanced), 2) => {
                self.flight.cloud_pixel_texture = !self.flight.cloud_pixel_texture
            }
            (Some(SettingsSection::Advanced), 3) => {
                self.flight.dynamic_lighting = !self.flight.dynamic_lighting
            }
            (Some(SettingsSection::Advanced), 4) => {
                self.flight.smooth_sky_gradient = !self.flight.smooth_sky_gradient
            }
            (Some(SettingsSection::Advanced), 5) => {
                self.flight.portal_preview = !self.flight.portal_preview
            }
            _ => return,
        }
        self.save_settings();
    }
    /// 保存全部设置；先写临时文件再改名，断电时不会留下损坏的设置文件
    pub(super) fn save_settings(&self) {
        let Some(path) = &self.settings_path else {
            return;
        };
        let data = format!(
            "faceted = {}\nportal_labels = {}\ndynamic_clouds = {}\ncloud_pixel_texture = {}\ndynamic_lighting = {}\nsmooth_sky_gradient = {}\nportal_preview = {}\nhud = {}\nvolume = {}\nsound_profile = {}\naim_assist = {}\ntilt_aim = {}\n",
            self.flight.faceted,
            self.portal_labels,
            self.flight.dynamic_clouds,
            self.flight.cloud_pixel_texture,
            self.flight.dynamic_lighting,
            self.flight.smooth_sky_gradient,
            self.flight.portal_preview,
            self.flight.hud,
            self.audio_settings.volume,
            self.audio_settings.profile,
            self.aim_assist,
            self.tilt_aim,
        );
        let result = (|| -> std::io::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let temp = path.with_extension("tmp");
            std::fs::write(&temp, data)?;
            std::fs::rename(temp, path)
        })();
        if let Err(error) = result {
            eprintln!("保存设置失败：{error}");
        }
    }
}
