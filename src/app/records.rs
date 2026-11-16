use super::*;

impl App {
    pub(super) fn record_key(&self) -> String {
        format!(
            "v2:{}:{}:{}:{}:{}:{}",
            self.mode as usize,
            self.difficulty as usize,
            self.flight.vehicle as usize,
            self.world.kind as usize,
            self.world.seed,
            self.level
        )
    }
    pub(super) fn save_record(&mut self) {
        if self.mode == Mode::Free || self.mission.score <= self.best_score {
            return;
        }
        if self.mode == Mode::Race && self.mission.outcome != Some(true) {
            return;
        }
        let key = self.record_key();
        self.new_record = true;
        self.best_score = self.mission.score;
        if let Some(record) = self.records.iter_mut().find(|r| r.0 == key) {
            record.1 = self.best_score;
        } else {
            self.records.push((key, self.best_score));
        }
        if let Some(path) = &self.record_path {
            let data = self
                .records
                .iter()
                .map(|(key, value)| format!("{key} {value}\n"))
                .collect::<String>();
            let result = (|| -> std::io::Result<()> {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let temp = path.with_extension("tmp");
                std::fs::write(&temp, data)?;
                std::fs::rename(temp, path)
            })();
            self.record_error = result.is_err();
        }
    }
}
