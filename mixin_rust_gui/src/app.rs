use std::{fs, path::PathBuf, sync::mpsc::Receiver, time::Instant};
use std::collections::HashMap;
use std::path::Path;
use eframe::{egui, Frame};
use eframe::egui::{Context, Ui};
use rfd::FileDialog;
use backend::backend::{find_manifest, BuildMessage, start_cargo_check, start_cargo_build, start_cargo_run};
use crate::{
    model::{BuildState, ModEntry, Page, Project},
    theme,
};
use crate::model::SignatureStatus;

pub struct MixinRustApp {
    logo: egui::TextureHandle,
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
        let logo = load_logo(&cc.egui_ctx);
        Self {
            logo,
            page: Page::Dashboard,
            project: Project::default(),
            mods: vec![],
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

        self.refresh_mods();
    }

    fn choose_project(&mut self) {
        if let Some(path) = FileDialog::new().set_title("Select Rust project").pick_file() {
            self.select_project(path);
        }
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
        self.page = Page::Log;
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
        self.page = Page::Log;
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

    fn mods_directory(&self) -> Option<PathBuf> {
        if self.project.path.is_none() {
            return None;
        }

        Some(data_dir(&self.project.name))
    }

    fn refresh_mods(&mut self) {
        let Some(directory) = self.mods_directory() else {
            self.mods.clear();
            self.selected_mod = 0;
            return;
        };

        if let Err(error) = fs::create_dir_all(&directory) {
            self.status_message = format!("Failed to create mod directory: {error}");
            self.mods.clear();
            self.selected_mod = 0;
            return;
        }

        let previous: HashMap<PathBuf, ModEntry> = self
            .mods
            .drain(..)
            .map(|modifier| (modifier.path.clone(), modifier))
            .collect();

        let mut mods = Vec::new();

        match fs::read_dir(&directory) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();

                    let is_mod = path.is_dir() || path.is_file();

                    if !is_mod {
                        continue;
                    }

                    if let Some(mut modifier) = previous.get(&path).cloned() {
                        modifier.name = path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Unnamed mod".into());

                        mods.push(modifier);
                        continue;
                    }

                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Unnamed mod".into());

                    let kind = if path.is_dir() {
                        "Folder mod"
                    } else {
                        "Archive mod"
                    };

                    mods.push(ModEntry {
                        id: format!("local:{name}"),
                        name,
                        version: "Unknown".into(),
                        author: "Unknown".into(),
                        description: kind.into(),
                        enabled: false,
                        path,
                        verification: SignatureStatus::Unsigned,
                        signature: None
                    });
                }
            }
            Err(error) => {
                self.status_message = format!("Failed to read mod directory: {error}");
            }
        }

        mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.mods = mods;

        if self.mods.is_empty() {
            self.selected_mod = 0;
        } else if self.selected_mod >= self.mods.len() {
            self.selected_mod = self.mods.len() - 1;
        }
    }

    fn select_mod_files(&mut self) {
        let Some(directory) = self.mods_directory() else {
            self.status_message = "Select a Rust project first".into();
            return;
        };

        let Some(paths) = FileDialog::new()
            .set_title("Select mod files")
            .pick_files()
        else {
            return;
        };

        let mut imported = 0usize;
        let mut failed = 0usize;

        for source in paths {
            match self.copy_mod(&source, &directory) {
                Ok(()) => imported += 1,
                Err(error) => {
                    eprintln!("[mods] {error}");
                    failed += 1;
                }
            }
        }

        self.refresh_mods();

        self.status_message = match (imported, failed) {
            (0, 0) => "No mods selected".into(),
            (imported, 0) => format!("Imported {imported} mod(s)"),
            (imported, failed) => format!("Imported {imported} mod(s), {failed} failed"),
            (0, failed) => format!("Failed to import {failed} mod(s)"),
        };
    }

    fn select_mod_folders(&mut self) {
        let Some(directory) = self.mods_directory() else {
            self.status_message = "Select a Rust project first".into();
            return;
        };

        let Some(paths) = FileDialog::new()
            .set_title("Select mod folders")
            .pick_folders()
        else {
            return;
        };

        let mut imported = 0usize;
        let mut failed = 0usize;

        for source in paths {
            match self.copy_mod(&source, &directory) {
                Ok(()) => imported += 1,
                Err(error) => {
                    eprintln!("[mods] {error}");
                    failed += 1;
                }
            }
        }

        self.refresh_mods();

        self.status_message = match (imported, failed) {
            (0, 0) => "No mods selected".into(),
            (imported, 0) => format!("Imported {imported} mod(s)"),
            (imported, failed) => format!("Imported {imported} mod(s), {failed} failed"),
            (0, failed) => format!("Failed to import {failed} mod(s)"),
        };
    }

    fn copy_mod(&self, source: &Path, destination_root: &Path) -> Result<(), String> {
        let name = source
            .file_name()
            .ok_or_else(|| format!("Invalid mod path: {}", source.display()))?;

        let destination = destination_root.join(name);

        if source == destination {
            return Err(format!("Mod is already installed: {}", source.display()));
        }

        if source.starts_with(destination_root) {
            return Err(format!(
                "Mod is already inside the project mod directory: {}",
                source.display()
            ));
        }

        if destination.exists() {
            return Err(format!(
                "A mod named '{}' already exists",
                name.to_string_lossy()
            ));
        }

        if source.is_dir() {
            copy_directory_recursive(source, &destination)
                .map_err(|error| format!("Failed to copy {}: {error}", source.display()))?;
        } else if source.is_file() {
            fs::copy(source, &destination)
                .map_err(|error| format!("Failed to copy {}: {error}", source.display()))?;
        } else {
            return Err(format!("Unsupported mod path: {}", source.display()));
        }

        Ok(())
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
                self.page = Page::Log;
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
            self.nav_button(ui, Page::Log, "Build Output");
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
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Dashboard");
                ui.label("Build a custom Rust source tree by combining the base project with enabled mixins.");
            });

            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Min),
                |ui| {
                    ui.allocate_ui(egui::vec2(220.0, 72.0), |ui| {
                        egui::Frame::NONE
                            .stroke(egui::Stroke::new(
                                1.0,
                                ui.visuals().widgets.noninteractive.fg_stroke.color,
                            ))
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Image::from_texture(&self.logo)
                                            .fit_to_exact_size(egui::vec2(64.0, 64.0)),
                                    );

                                    ui.vertical_centered(|ui| {
                                        ui.label(egui::RichText::new("Rusty Skylines").strong());
                                        ui.label("Alpha v1.9.0a");
                                        ui.label(egui::RichText::new("by maxwag9").weak());
                                    });
                                });
                            });
                    });
                },
            );

        });

        ui.add_space(14.0);

        // ui.heading("Dashboard");
        // ui.label("Build a custom Rust source tree by combining the base project with enabled mixins.");
        // ui.add_space(14.0);

        egui::Grid::new("dashboard_cards")
            .num_columns(2)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                self.metric_card(ui, "Project:", &self.project.name, "");
                self.metric_card(ui, "Active Mods:", &self.active_mods().to_string(), "");
                ui.end_row();
                self.metric_card(ui, "Hard Mods:", &self.active_mods().to_string(), "");
                self.metric_card(ui, "Status:", &self.status_message, "");
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
                    ui.label(egui::RichText::new(format!("{}", modifier.verification)).weak());
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
        self.refresh_mods();

        ui.horizontal(|ui| {
            ui.heading("Mods");

            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    let mut pick_files = false;
                    let mut pick_folders = false;

                    ui.menu_button("Select Mods", |ui| {
                        if ui.button("Select mod files").clicked() {
                            pick_files = true;
                        }

                        if ui.button("Select mod folders").clicked() {
                            pick_folders = true;
                        }
                    });

                    if ui.button("Refresh").clicked() {
                        self.refresh_mods();
                    }

                    if pick_files {
                        self.select_mod_files();
                    }

                    if pick_folders {
                        self.select_mod_folders();
                    }
                },
            );
        });

        ui.label(
            egui::RichText::new(
                "Mods are stored in the project's Mixin Rust data directory.",
            )
                .weak(),
        );

        ui.add_space(12.0);

        if self.project.path.is_none() {
            ui.label("Select a Rust project first.");
            return;
        }

        if self.mods.is_empty() {
            theme::section_frame(ui.style()).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                    ui.label("No mods installed.");
                    ui.label(
                        egui::RichText::new("Use Select Mods to add folders or archive files.")
                            .weak(),
                    );
                    ui.add_space(12.0);
                });
            });
            return;
        }

        ui.columns(2, |columns| {
            columns[0].vertical(|ui| {
                for index in 0..self.mods.len() {
                    let modifier = &self.mods[index];
                    let selected = self.selected_mod == index;

                    let mut label = format!("{}  ·  {}", modifier.name, modifier.version);

                    if matches!(
                    modifier.verification,
                    SignatureStatus::Trusted | SignatureStatus::Valid
                ) {
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

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.checkbox(&mut modifier.enabled, "Enabled");
                            },
                        );
                    });

                    ui.label(format!("{}", modifier.version));
                    ui.label(format!("by {}", modifier.author));

                    ui.add_space(8.0);
                    ui.label(&modifier.description);

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.monospace(&modifier.id);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Signature:");
                        ui.label(modifier.verification.to_string());
                    });

                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        ui.monospace(modifier.path.display().to_string());
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
            .resizable(true)
            .default_size(150.0)
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
                    Page::Log => self.build_page(ui),
                    Page::Settings => self.settings_page(ui),
                });
        });
    }
}


fn load_logo(ctx: &egui::Context) -> egui::TextureHandle {

    let image = image::open(data_dir("textures/city_road.png"))
        .expect("Failed to load logo")
        .to_rgba8();

    let size = [image.width() as usize, image.height() as usize];

    ctx.load_texture(
        "logo",
        egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::NEAREST
    )
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .expect("Failed to get executable path")
        .parent()
        .expect("Executable has no parent directory")
        .to_path_buf()
}

/// Finds the data directory by searching multiple candidate locations.
fn find_data_root() -> PathBuf {
    let exe = exe_dir();

    // Check candidates in priority order
    let candidates: &[PathBuf] = &[
        exe.join("data"),          // Distribution: data/ beside exe
        exe.join("../data"),       // Distribution: data/ one level up
        exe.join("../../data"),    // Dev: target/release/ or target/debug/
        exe.join("../../../data"), // Dev: nested workspace crate
    ];

    for candidate in candidates {
        if candidate.is_dir() {
            // Canonicalize resolves ".." and returns absolute path
            if let Ok(resolved) = candidate.canonicalize() {
                println!("[data_path] Found data directory: {}", resolved.display());
                return resolved;
            }
        }
    }

    // Log what we tried (helps debugging distribution issues)
    eprintln!("[data_path] ERROR: Could not find data directory!");
    eprintln!("[data_path] Executable directory: {}", exe.display());
    eprintln!("[data_path] Searched:");
    for c in candidates {
        eprintln!("  - {} (exists: {})", c.display(), c.exists());
    }

    // Return most likely distribution path for meaningful error messages
    exe.join("data")
}
fn data_root() -> &'static PathBuf {
    use std::sync::OnceLock;
    static DATA_ROOT: OnceLock<PathBuf> = OnceLock::new();
    DATA_ROOT.get_or_init(find_data_root)
}

pub fn data_dir(path: impl AsRef<Path>) -> PathBuf {
    data_root().join(path.as_ref())
}
fn copy_directory_recursive(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else if source_path.is_file() {
            fs::copy(&source_path, &destination_path)?;
        }
    }

    Ok(())
}