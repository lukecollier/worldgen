#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![feature(portable_simd)]

use std::{
    hash::{DefaultHasher, Hasher},
    ops::RangeInclusive,
    sync::{mpsc, Arc, RwLock},
    thread,
};

use worldgen::gen::Gen;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "WorldGen",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::<App>::default()
        }),
    )
}

enum Message {
    Generate,
}

struct App {
    preview: Arc<RwLock<Vec<u8>>>,
    generator: Arc<RwLock<Gen>>,
    messages: mpsc::Sender<Message>,
    last_preview_hash: u64,
}

impl Default for App {
    fn default() -> Self {
        let mut buf = vec![0; 2048 * 2048];
        let generator = Gen::default();
        generator.gen(&mut buf);
        let (tx, rx) = mpsc::channel();
        let data = Arc::from(RwLock::from(buf));
        let rw_generator = Arc::from(RwLock::from(generator));
        let data_arc = Arc::clone(&data);
        let generator_arc = Arc::clone(&rw_generator);
        let app = Self {
            preview: data,
            generator: rw_generator,
            messages: tx,
            last_preview_hash: 0,
        };
        // Our background processor thread so the UI is never blocked
        thread::spawn(move || {
            let mut buf = vec![0; 2048 * 2048];
            for msg in rx {
                match msg {
                    Message::Generate => {
                        generator_arc.read().unwrap().gen(&mut buf);
                        data_arc.write().unwrap().copy_from_slice(&buf);
                    }
                }
            }
        });
        app
    }
}

impl App {
    fn hash_preview(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hasher.write(&self.preview.read().unwrap());
        hasher.finish()
    }
}

fn slider<'a, Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut Num,
    range: RangeInclusive<Num>,
    text: &str,
    on_release: impl Fn(Num) + 'a,
) -> egui::Response {
    let slider = egui::Slider::new(value, range).text(text);
    let response = ui.add(slider);
    if response.drag_released() || response.lost_focus() || response.changed() {
        on_release(*value);
    };
    response
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            slider(
                ui,
                &mut self.generator.write().unwrap().frequency,
                0.0..=1.0,
                "frequency",
                |_| {
                    self.messages.send(Message::Generate).unwrap();
                },
            );
            slider(
                ui,
                &mut self.generator.write().unwrap().octaves,
                0..=30,
                "octaves",
                |_| {
                    self.messages.send(Message::Generate).unwrap();
                },
            );
            slider(
                ui,
                &mut self.generator.write().unwrap().persistence,
                -0.6..=0.6,
                "persistence",
                |_| {
                    self.messages.send(Message::Generate).unwrap();
                },
            );
            slider(
                ui,
                &mut self.generator.write().unwrap().lacunarity,
                -1.0..=3.0,
                "lacunarity",
                |_| {
                    self.messages.send(Message::Generate).unwrap();
                },
            );
            slider(
                ui,
                &mut self.generator.write().unwrap().seed,
                0..=100,
                "seed",
                |_| {
                    self.messages.send(Message::Generate).unwrap();
                },
            );

            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(egui::Image::from_bytes(
                    "bytes://preview.png",
                    self.preview.read().expect("don't die").clone(),
                ));
            });
            // if we have a new preview, forget the old one
            if self.hash_preview() != self.last_preview_hash {
                ctx.forget_image("bytes://preview.png");
                self.last_preview_hash = self.hash_preview();
            }
        });
    }
}
