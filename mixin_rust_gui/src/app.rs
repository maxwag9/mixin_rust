use std::{path::PathBuf, sync::mpsc::Receiver, time::Instant};

use eframe::{egui, Frame};
use eframe::egui::{Context, Ui};
use rfd::FileDialog;
use backend::backend::{find_manifest, BuildMessage, start_cargo_check, start_cargo_build, start_cargo_run};
use crate::{
    model::{BuildState, ModEntry, Page, Project},
    theme,
};

pub struct MixinRustApp {
    page: Page,
    project: Project,
    mods: Vec<ModEntry>,
    selected_mod: usize,
    build_state: BuildState,
    build_receiver: Option<Receiver<BuildMessage>>,
    build_logs: Vec<String>,
    auto_scroll_logs: bool,
    last_action: Option<String>,
    status_message: String,
    dark_mode: bool,
    compact_mode: bool,
    build_started: Option<Instant>,
}

impl MixinRustApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx, false, true);

        Self {
            page: Page::Dashboard,
            project: Project::default(),
            mods: ModEntry::demo_data(),
            selected_mod: 0,
            build_state: BuildState::Idle,
            build_receiver: None,
            build_logs: Vec::new(),
            auto_scroll_logs: true,
            last_action: None,
            status_message: "Ready".into(),
            dark_mode: true,
            compact_mode: false,
            build_started: None,
        }
    }

    fn select_project(&mut self, path: PathBuf) {
        let manifest = find_manifest(&path);

        self.project.path = manifest
            .as_ref()
            .and_then(|manifest| manifest.parent().map(PathBuf::from));
        self.project.cargo_manifest = manifest;

        self.project.name = self
            .project
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unknown project".into());

        self.project.version = "Cargo.toml not parsed yet".into();
        self.status_message = if self.project.cargo_manifest.is_some() {
            "Rust project selected".into()
        } else {
            "Selected folder does not contain Cargo.toml".into()
        };
    }

    fn choose_project(&mut self) {
        if let Some(path) = FileDialog::new().set_title("Select Rust project").pick_folder() {
            self.select_project(path);
        }
    }

    fn active_hard_mods(&self) -> usize {
        self.mods
            .iter()
            .filter(|m| m.enabled && matches!(m.kind, crate::model::ModKind::Hard))
            .count()
    }

    fn active_mods(&self) -> usize {
        self.mods.iter().filter(|m| m.enabled).count()
    }

    fn start_check(&mut self) {
        let Some(path) = self.project.path.clone() else {
            self.status_message = "Select a Rust project first".into();
            return;
        };

        self.build_logs.clear();
        self.build_logs.push("Starting cargo check --release".into());
        self.build_state = BuildState::Building;
        self.build_started = Some(Instant::now());
        self.status_message = "Checking project".into();
        self.build_receiver = Some(start_cargo_check(&path).receiver);
        self.page = Page::Build;
    }

    fn start_build(&mut self) {
        let Some(path) = self.project.path.clone() else {
            self.status_message = "Select a Rust project first".into();
            return;
        };

        self.build_logs.clear();
        self.build_logs.push("Starting cargo build --release".into());
        self.build_state = BuildState::Building;
        self.build_started = Some(Instant::now());
        self.status_message = "Building project".into();
        self.build_receiver = Some(start_cargo_build(&path).receiver);
        self.page = Page::Build;
    }

    fn poll_build(&mut self) {
        let Some(receiver) = self.build_receiver.as_ref() else {
            return;
        };

        let mut finished = None;
        while let Ok(message) = receiver.try_recv() {
            match message {
                BuildMessage::Line(line) => self.build_logs.push(line),
                BuildMessage::Finished(success) => finished = Some(success),
            }
        }

        if let Some(success) = finished {
            self.build_state = if success {
                BuildState::Succeeded
            } else {
                BuildState::Failed
            };
            self.status_message = if success {
                "Build completed successfully".into()
            } else {
                "Build failed".into()
            };
            self.build_receiver = None;
        }
    }

    fn run_game(&mut self) {
        let Some(path) = self.project.path.clone() else {
            self.status_message = "Select a Rust project first".into();
            return;
        };

        match start_cargo_run(&path) {
            Ok(()) => {
                self.last_action = Some("Started cargo run --release".into());
                self.status_message = "Game launched".into();
            }
            Err(error) => {
                self.status_message = error;
                self.page = Page::Build;
            }
        }
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Mixin Rust");
            ui.separator();
            ui.label(egui::RichText::new("Native Rust modding workbench").weak());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_run = self.project.path.is_some() && self.build_state == BuildState::Succeeded;
                if ui
                    .add_enabled(can_run, egui::Button::new("▶ Run"))
                    .clicked()
                {
                    self.run_game();
                }

                let can_build = self.project.path.is_some() && self.build_state != BuildState::Building;
                if ui.add_enabled(can_build, egui::Button::new("Build")).clicked() {
                    self.start_build();
                }

                if ui.button("Choose Project").clicked() {
                    self.choose_project();
                }
            });
        });
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            self.nav_button(ui, Page::Dashboard, "Dashboard");
            self.nav_button(ui, Page::Mods, "Mods");
            self.nav_button(ui, Page::Build, "Build Output");
            self.nav_button(ui, Page::Settings, "Settings");

            ui.add_space(18.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(egui::RichText::new("PROJECT").small().strong().weak());
            ui.label(&self.project.name);
            ui.label(
                egui::RichText::new(
                    self.project
                        .path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "No project selected".into()),
                )
                .small()
                .weak(),
            );

            ui.add_space(16.0);
            ui.label(egui::RichText::new("MODS").small().strong().weak());
            ui.label(format!("{} active", self.active_mods()));
            ui.label(format!("{} hard", self.active_hard_mods()));
        });
    }

    fn nav_button(&mut self, ui: &mut Ui, page: Page, label: &str) {
        let selected = self.page == page;
        let response = ui.add_sized(
            [ui.available_width(), 34.0],
            egui::Button::selectable(selected, label),
        );
        if response.clicked() {
            self.page = page;
        }
    }

    fn dashboard(&mut self, ui: &mut Ui) {
        ui.heading("Dashboard");
        ui.label("Build a custom Rust source tree by combining the base project with enabled mixins.");
        ui.add_space(14.0);

        egui::Grid::new("dashboard_cards")
            .num_columns(2)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                self.metric_card(ui, "Project", &self.project.name, "Cargo project");
                self.metric_card(ui, "Active Mods", &self.active_mods().to_string(), "All mod types");
                ui.end_row();
                self.metric_card(ui, "Hard Mods", &self.active_hard_mods().to_string(), "Source transformations");
                self.metric_card(ui, "Build Status", self.build_state.label(), &self.status_message);
            });

        ui.add_space(18.0);
        theme::section_frame(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Quick Actions");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Manage Mods").clicked() {
                        self.page = Page::Mods;
                    }
                    if ui.button("Check").clicked() {
                        self.start_check();
                    }
                    if ui.button("Build").clicked() {
                        self.start_build();
                    }
                });
            });
            ui.separator();
            ui.label("The real patch engine will plug into this layer later. For now the skeleton can check/build any Cargo project.");
        });

        ui.add_space(18.0);
        theme::section_frame(ui.style()).show(ui, |ui| {
            ui.heading("Active Mixins");
            ui.add_space(6.0);
            for modifier in self.mods.iter().filter(|m| m.enabled) {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&modifier.name).strong());
                    ui.label(modifier.kind.label());
                    if modifier.trusted {
                        ui.label(egui::RichText::new("Trusted").weak());
                    }
                });
            }
        });
    }

    fn metric_card(&self, ui: &mut Ui, title: &str, value: &str, subtitle: &str) {
        theme::section_frame(ui.style()).show(ui, |ui| {
            ui.set_min_width(270.0);
            ui.label(egui::RichText::new(title).small().weak());
            ui.add_space(4.0);
            ui.label(egui::RichText::new(value).size(22.0).strong());
            ui.label(egui::RichText::new(subtitle).small().weak());
        });
    }

    fn mods_page(&mut self, ui: &mut Ui) {
        ui.heading("Mods");
        ui.label("Toggle the mixins that will participate in the next generated build.");
        ui.add_space(12.0);

        ui.columns(2, |columns| {
            columns[0].vertical(|ui| {
                for index in 0..self.mods.len() {
                    let modifier = &self.mods[index];
                    let selected = self.selected_mod == index;
                    let mut label = format!("{}  ·  {}", modifier.name, modifier.version);
                    if modifier.trusted {
                        label.push_str("  ✓");
                    }

                    let response = ui.add_sized(
                        [ui.available_width(), 42.0],
                        egui::Button::selectable(selected, label),
                    );
                    if response.clicked() {
                        self.selected_mod = index;
                    }
                }
            });

            columns[1].vertical(|ui| {
                let Some(modifier) = self.mods.get_mut(self.selected_mod) else {
                    return;
                };

                theme::section_frame(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(&modifier.name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut modifier.enabled, "Enabled");
                        });
                    });

                    ui.label(format!("{} {}", modifier.kind.label(), modifier.version));
                    ui.label(format!("by {}", modifier.author));
                    ui.add_space(8.0);
                    ui.label(&modifier.description);
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.monospace(&modifier.id);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Trust:");
                        ui.label(if modifier.trusted { "Trusted" } else { "Untrusted" });
                    });
                });
            });
        });
    }

    fn build_page(&mut self, ui: &mut Ui) {
        ui.heading("Build Output");
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(self.build_state.label()).strong());
            if let Some(started) = self.build_started {
                if self.build_state == BuildState::Building {
                    ui.label(format!("{:.1}s", started.elapsed().as_secs_f32()));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.auto_scroll_logs, "Auto-scroll");
                if ui.button("Clear").clicked() {
                    self.build_logs.clear();
                }
                if ui
                    .add_enabled(self.project.path.is_some() && self.build_state != BuildState::Building, egui::Button::new("Check"))
                    .clicked()
                {
                    self.start_check();
                }
                if ui
                    .add_enabled(self.project.path.is_some() && self.build_state != BuildState::Building, egui::Button::new("Build"))
                    .clicked()
                {
                    self.start_build();
                }
            });
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .stick_to_bottom(self.auto_scroll_logs)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for line in &self.build_logs {
                    ui.monospace(line);
                }
            });
    }

    fn settings_page(&mut self, ui: &mut Ui) {
        ui.heading("Settings");
        ui.add_space(8.0);

        theme::section_frame(ui.style()).show(ui, |ui| {
            ui.heading("Interface");
            ui.checkbox(&mut self.dark_mode, "Dark mode");
            ui.checkbox(&mut self.compact_mode, "Compact layout");
        });

        ui.add_space(12.0);
        theme::section_frame(ui.style()).show(ui, |ui| {
            ui.heading("Build defaults");
            ui.label("These controls are placeholders for the future build/profile configuration.");
            ui.horizontal(|ui| {
                ui.label("Profile");
                egui::ComboBox::from_id_salt("build_profile")
                    .selected_text("release")
                    .show_ui(ui, |ui| {
                        ui.selectable_label(true, "release");
                        ui.selectable_label(false, "debug");
                    });
            });
            ui.label("Profile switching will become its own project setting later.");
        });

        ui.add_space(12.0);
        theme::section_frame(ui.style()).show(ui, |ui| {
            ui.heading("Project");
            if ui.button("Choose Rust project").clicked() {
                self.choose_project();
            }
            ui.monospace(
                self.project
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<none>".into()),
            );
        });
    }
}

impl eframe::App for MixinRustApp {
    fn logic(&mut self, _ctx: &Context, _frame: &mut Frame) {

    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        theme::apply(ui, self.compact_mode, self.dark_mode);
        self.poll_build();
        if self.build_state == BuildState::Building {
            ui.request_repaint_after(std::time::Duration::from_millis(50));
        }

        egui::Panel::top("top_bar")
            .resizable(false)
            .show(ui, |ui| {
                self.top_bar(ui);
            });

        egui::Panel::left("sidebar")
            .resizable(false)
            .default_size(210.0)
            .show(ui, |ui| {
                self.sidebar(ui);
            });

        egui::Panel::bottom("status_bar")
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status_message);
                    if let Some(last_action) = &self.last_action {
                        ui.separator();
                        ui.label(last_action);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("Mixin Rust 0.1.0").small().weak());
                    });
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.page {
                    Page::Dashboard => self.dashboard(ui),
                    Page::Mods => self.mods_page(ui),
                    Page::Build => self.build_page(ui),
                    Page::Settings => self.settings_page(ui),
                });
        });
    }
}
