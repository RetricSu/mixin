//! Headless checks of the same UI used by the native window, with ephemeral identities.
use super::*;

fn fixture(identity: &(String, String)) -> App {
    App::with_identity(identity.clone(), Vec::new(), false)
}

#[test]
fn send_validation_and_receive_result_lifecycle() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    assert!(app.send_blocker().is_some());
    app.send_text = "请于八点见面".into();
    app.send_password = "test-passphrase".into();
    app.send_confirm = "different".into();
    assert!(app.send_blocker().is_some());
    app.send_confirm = app.send_password.clone();
    assert!(app.send_blocker().is_none());

    // A text draft must not substitute for a missing file in file mode.
    app.payload_type = PayloadType::File;
    app.do_send();
    assert!(app.generated_cipher.is_empty());
    app.payload_type = PayloadType::Text;
    app.do_send();
    assert!(app.generated_cipher.starts_with(PREFIX));
    app.receive_input = app.generated_cipher.clone();
    app.receive_password = app.send_password.clone();
    app.do_receive();
    assert!(matches!(&app.decrypted, Some(DecryptedContent::Text(text)) if text == &app.send_text));

    app.receive_password = "wrong".into();
    app.do_receive();
    assert!(
        app.decrypted.is_none(),
        "A failed attempt must discard the old result"
    );

    app.send_mode = SendMode::Contact;
    app.recipient_custom_key = "incomplete public identity".into();
    assert!(app.send_blocker().is_some());
    app.recipient_custom_key = identity.1.clone();
    assert!(app.send_blocker().is_none());
    app.do_send();
    app.receive_input = app.generated_cipher.clone();
    app.do_receive();
    assert!(matches!(&app.decrypted, Some(DecryptedContent::Text(text)) if text == &app.send_text));
}

fn render(
    app: &mut App,
    ctx: &egui::Context,
    width: f32,
    height: f32,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            events,
            ..Default::default()
        },
        |ctx| app.render_ui(ctx),
    )
}

#[test]
fn layouts_fit_native_window_sizes_in_both_themes() {
    let identity = generate_identity().unwrap();
    for dark in [false, true] {
        for (width, height) in [(720.0, 520.0), (780.0, 540.0), (800.0, 640.0), (1040.0, 800.0)] {
            for state in [
                "send",
                "file",
                "selected-file",
                "decrypted-text",
                "decrypted-file",
                "contact",
                "generated",
                "receive",
                "password",
                "identity",
                "add-contact",
            ] {
                let ctx = egui::Context::default();
                theme::configure_fonts(&ctx);
                theme::apply_theme(&ctx, dark);
                let mut app = fixture(&identity);
                app.dark_mode = dark;
                match state {
                    "file" => app.payload_type = PayloadType::File,
                    "selected-file" => {
                        app.payload_type = PayloadType::File;
                        app.send_file = Some(SelectedFile {
                            path: PathBuf::new(),
                            name: "季度报告-".repeat(16),
                            extension: "pdf".into(),
                            size: 256,
                            bytes: vec![0; 256],
                        });
                    }
                    "decrypted-text" => {
                        app.tab = Tab::Receive;
                        app.receive_input = "MX1.preview".into();
                        app.decrypted = Some(DecryptedContent::Text(
                            "这是用于布局检查的解密文字。".repeat(30),
                        ));
                    }
                    "decrypted-file" => {
                        app.tab = Tab::Receive;
                        app.receive_input = "MX1.preview".into();
                        app.decrypted = Some(DecryptedContent::File {
                            meta: FileMeta {
                                name: "特别长的文件名称-".repeat(12),
                                extension: "zip".into(),
                                content_type: "application/zip".into(),
                            },
                            bytes: vec![0; 256],
                            saved_path: None,
                        });
                    }
                    "contact" => app.send_mode = SendMode::Contact,
                    "generated" => {
                        app.send_text = "界面检查使用的临时内容".into();
                        app.send_password = "preview-only".into();
                        app.send_confirm = app.send_password.clone();
                        app.do_send();
                        app.toast = None;
                    }
                    "receive" => app.tab = Tab::Receive,
                    "password" => {
                        app.tab = Tab::Receive;
                        app.receive_input = "MX1.preview".into();
                    }
                    "identity" | "add-contact" => {
                        app.tab = Tab::Identity;
                        app.show_add_contact_dialog = state == "add-contact";
                        app.contacts.push(Contact {
                            name: "测试联系人".into(),
                            public_key: identity.1.clone(),
                            fingerprint: compute_fingerprint(&identity.1),
                            created_at: String::new(),
                        });
                    }
                    _ => {}
                }
                // Allow immediate-mode layout and font atlas to settle before inspecting.
                let first = render(&mut app, &ctx, width, height, vec![]);
                let output = render(&mut app, &ctx, width, height, vec![]);
                if matches!(state, "send" | "file" | "contact" | "selected-file") {
                    text_center(&output, "生成密信");
                    if state != "contact" {
                        text_center(&output, "解密口令");
                        text_center(&output, "确认口令");
                        text_center(&output, "随机生成并复制");
                    }
                }
                for shape in &output.shapes {
                    if let egui::Shape::Text(text) = &shape.shape {
                        let rect = text.galley.rect.translate(text.pos.to_vec2());
                        if shape.clip_rect.intersects(rect) {
                            assert!(rect.right() <= width + 1.0,
                                "{state}, dark={dark}, {width}: horizontal overflow: {} at {rect:?}", text.galley.job.text);
                        }
                    }
                }
                if let Ok(dir) = std::env::var("MIXIN_REVIEW_DIR") {
                    export_mesh(
                        &ctx,
                        first,
                        output,
                        width,
                        height,
                        &PathBuf::from(dir).join(format!(
                            "{state}-{}-{}.json",
                            if dark { "dark" } else { "light" },
                            width as u32
                        )),
                    );
                }
            }
        }
    }
}

#[test]
fn navigation_responds_to_pointer_and_keyboard() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let point = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                if text.galley.job.text == "接收密信" {
                    return Some(text.galley.rect.translate(text.pos.to_vec2()).center());
                }
            }
            None
        })
        .expect("Receive navigation is visible");
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.tab == Tab::Receive);
    // Native egui buttons must participate in keyboard focus order.
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(ctx.memory(|m| m.focused().is_some()));
}

// Optional review artifact export; the normal test run performs no filesystem writes.
fn export_mesh(
    ctx: &egui::Context,
    first: egui::FullOutput,
    output: egui::FullOutput,
    width: f32,
    height: f32,
    path: &std::path::Path,
) {
    let mut textures = Vec::new();
    for (id, delta) in first
        .textures_delta
        .set
        .iter()
        .chain(output.textures_delta.set.iter())
    {
        let pixels: Vec<[u8; 4]> = match &delta.image {
            egui::ImageData::Color(image) => image.pixels.iter().map(|p| p.to_array()).collect(),
            egui::ImageData::Font(image) => {
                image.srgba_pixels(None).map(|p| p.to_array()).collect()
            }
        };
        textures.push(serde_json::json!({"id": format!("{id:?}"), "size": delta.image.size(), "pos": delta.pos, "pixels": pixels}));
    }
    let meshes: Vec<_> = ctx.tessellate(output.shapes, 1.0).into_iter().filter_map(|primitive| {
        if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
            Some(serde_json::json!({"clip": [primitive.clip_rect.min.x, primitive.clip_rect.min.y, primitive.clip_rect.max.x, primitive.clip_rect.max.y],
                "texture": format!("{:?}", mesh.texture_id), "indices": mesh.indices,
                "vertices": mesh.vertices.iter().map(|v| [v.pos.x, v.pos.y, v.uv.x, v.uv.y, v.color.r() as f32, v.color.g() as f32, v.color.b() as f32, v.color.a() as f32]).collect::<Vec<_>>() }))
        } else { None }
    }).collect();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&serde_json::json!({"width": width, "height": height, "textures": textures, "meshes": meshes})).unwrap()).unwrap();
}

fn text_center(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                let rect = text.galley.rect.translate(text.pos.to_vec2());
                if text.galley.job.text == label && shape.clip_rect.contains_rect(rect) {
                    return Some(rect.center());
                }
            }
            None
        })
        .unwrap_or_else(|| panic!("Expected fully visible control: {label}"))
}

fn click(app: &mut App, ctx: &egui::Context, point: egui::Pos2) {
    for pressed in [true, false] {
        render(
            app,
            ctx,
            1040.0,
            800.0,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn generate_and_return_to_edit_preserves_draft() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    app.send_text = "保留这份草稿".into();
    app.send_password = "test-passphrase".into();
    app.send_confirm = app.send_password.clone();
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);
    render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    // All password controls fit the default viewport without scrolling.
    text_center(&output, "随机生成并复制");
    let point = text_center(&output, "生成密信");
    click(&mut app, &ctx, point);
    assert!(!app.generated_cipher.is_empty());
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let point = text_center(&output, "返回编辑");
    click(&mut app, &ctx, point);
    assert!(app.generated_cipher.is_empty());
    assert_eq!(app.send_text, "保留这份草稿");
    assert_eq!(app.send_password, "test-passphrase");

    // The paper toolbar remains interactive after returning from the result.
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    click(&mut app, &ctx, text_center(&output, "文件"));
    assert!(app.payload_type == PayloadType::File);
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "选择文件");
    click(&mut app, &ctx, text_center(&output, "文字"));
    assert!(app.payload_type == PayloadType::Text);
    assert_eq!(app.send_text, "保留这份草稿");
}

#[test]
fn theme_text_contrast_and_heading_colors() {
    fn luminance(color: Color32) -> f32 {
        let channel = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }
    fn contrast(a: Color32, b: Color32) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    for (dark, theme) in [(false, theme::LIGHT_THEME), (true, theme::DARK_THEME)] {
        for text in [
            theme.text_primary,
            theme.text_secondary,
            theme.text_muted,
            theme.text_accent,
        ] {
            assert!(
                [
                    theme.bg_canvas,
                    theme.bg_header,
                    theme.bg_well,
                    theme.bg_card
                ]
                .into_iter()
                .all(|background| contrast(text, background) >= 4.5),
                "Insufficient text contrast: {text:?}, dark={dark}"
            );
        }
        assert!(contrast(theme.text_accent, theme.bg_selection) >= 4.5);
        assert!(contrast(theme.text_on_accent, theme.accent_emerald) >= 4.5);
        let ctx = egui::Context::default();
        theme::apply_theme(&ctx, dark);
        assert_eq!(ctx.style().visuals.strong_text_color(), theme.text_primary);
    }
}
