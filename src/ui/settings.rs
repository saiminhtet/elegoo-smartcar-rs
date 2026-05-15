/// Settings dialog - IP, language, theme, etc.
#[derive(Clone)]
pub struct SettingsState {
    pub ip_address: String,
    pub port: String,
    pub language: Language,
    pub dark_theme: bool,
}

#[derive(Clone, PartialEq)]
pub enum Language {
    English,
    Russian,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            ip_address: "192.168.4.1".to_string(),
            port: "100".to_string(),
            language: Language::English,
            dark_theme: true,
        }
    }
}

impl SettingsState {
    pub fn joystick_max_speed(&self) -> u8 {
        200
    }
}

/// Show the settings window
pub fn show_settings(ctx: &egui::Context, settings: &mut SettingsState, open: &mut bool) {
    let mut close = false;
    egui::Window::new("Settings")
        .id(egui::Id::new("settings_window"))
        .open(open)
        .default_size([350.0, 300.0])
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Connection");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("IP Address:");
                ui.text_edit_singleline(&mut settings.ip_address);
            });

            ui.horizontal(|ui| {
                ui.label("Port:");
                ui.text_edit_singleline(&mut settings.port);
            });

            ui.add_space(12.0);
            ui.heading("Preferences");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Language:");
                egui::ComboBox::from_label("Select")
                    .selected_text(match settings.language {
                        Language::English => "English",
                        Language::Russian => "Русский",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.language, Language::English, "English");
                        ui.selectable_value(&mut settings.language, Language::Russian, "Русский");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Theme:");
                ui.checkbox(&mut settings.dark_theme, "Dark Mode");
            });

            ui.add_space(12.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Save & Reconnect").clicked() {
                    tracing::info!("Settings saved: IP={}, Port={}", settings.ip_address, settings.port);
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    if close {
        *open = false;
    }
}
