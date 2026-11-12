//! Headless checks of the same UI used by the native window, with ephemeral identities.
use super::*;
use eframe::egui::Color32;

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
        for (width, height) in [
            (720.0, 520.0),
            (780.0, 540.0),
            (800.0, 640.0),
            (1040.0, 800.0),
        ] {
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
                if std::env::var("WAXSEAL_REVIEW_DIR")
                    .or_else(|_| std::env::var("MIXIN_REVIEW_DIR"))
                    .is_ok()
                {
                    ctx.set_pixels_per_point(2.0);
                }
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
                if let Ok(dir) =
                    std::env::var("WAXSEAL_REVIEW_DIR").or_else(|_| std::env::var("MIXIN_REVIEW_DIR"))
                {
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
    let meshes: Vec<_> = ctx.tessellate(output.shapes, ctx.pixels_per_point()).into_iter().filter_map(|primitive| {
        if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
            Some(serde_json::json!({"clip": [primitive.clip_rect.min.x, primitive.clip_rect.min.y, primitive.clip_rect.max.x, primitive.clip_rect.max.y],
                "texture": format!("{:?}", mesh.texture_id), "indices": mesh.indices,
                "vertices": mesh.vertices.iter().map(|v| [v.pos.x, v.pos.y, v.uv.x, v.uv.y, v.color.r() as f32, v.color.g() as f32, v.color.b() as f32, v.color.a() as f32]).collect::<Vec<_>>() }))
        } else { None }
    }).collect();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&serde_json::json!({"width": width, "height": height, "ppp": ctx.pixels_per_point(), "textures": textures, "meshes": meshes})).unwrap()).unwrap();
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

#[test]
fn file_encryption_and_decryption_ui_lifecycle() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);

    // 1. Send file setup
    let dummy_file_bytes = b"PDF-1.7 confidential report binary content \x00\xFF\xFE\x01\x02";
    app.payload_type = PayloadType::File;
    app.send_file = Some(SelectedFile {
        path: PathBuf::from("/tmp/annual_report.pdf"),
        name: "annual_report".into(),
        extension: "pdf".into(),
        size: dummy_file_bytes.len(),
        bytes: dummy_file_bytes.to_vec(),
    });
    app.send_password = "secure-file-pass".into();
    app.send_confirm = "secure-file-pass".into();

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);

    // Render send UI and verify file action
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "生成密信");
    app.do_send();
    assert!(!app.generated_cipher.is_empty());

    // In generated output for file: "保存密信文件" must be visible, but "查看加密文本" must NOT be visible
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "保存密信文件");
    assert!(
        !output.shapes.iter().any(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                text.galley.job.text.contains("查看加密文本")
            } else {
                false
            }
        }),
        "Raw ciphertext drawer must be hidden for files to avoid user fear"
    );

    // 2. Receive file: simulate loading .seal cipher file
    let temp_cipher_path = std::env::temp_dir().join("annual_report.pdf.seal");
    fs::write(&temp_cipher_path, &app.generated_cipher).unwrap();
    app.load_cipher_file_for_receive(temp_cipher_path.clone());
    let _ = fs::remove_file(&temp_cipher_path);

    assert!(app.tab == Tab::Receive);
    assert!(app.receive_file.is_some());
    assert!(app.decrypted.is_none());

    // When a cipher file is loaded, the UI renders the friendly file card, NOT a raw multiline ciphertext edit
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "更换文件");
    text_center(&output, "移除");
    assert!(
        output.shapes.iter().any(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                text.galley.job.text.contains("annual_report.pdf.seal")
            } else {
                false
            }
        }),
        "Loaded cipher file name must be displayed"
    );

    // 3. Decrypt with correct password
    app.receive_password = "secure-file-pass".into();
    app.do_receive();

    // Verify it is DecryptedContent::File, NOT bare DecryptedContent::Text
    match &app.decrypted {
        Some(DecryptedContent::File {
            meta,
            bytes,
            saved_path,
        }) => {
            assert_eq!(meta.name, "annual_report");
            assert_eq!(meta.extension, "pdf");
            assert_eq!(bytes, dummy_file_bytes);
            assert!(saved_path.is_none());
        }
        Some(DecryptedContent::Text(_)) => {
            panic!("File decryption must NEVER produce bare decrypted text!")
        }
        None => panic!("Decryption failed"),
    }

    // 4. In decrypted state: UI cleanly displays file result card and "保存文件"
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "文件解密成功");
    text_center(&output, "保存文件");
    text_center(&output, "立即销毁 / 隐藏密文");

    // Simulate saving file: saved actions appear
    let saved_dest = std::env::temp_dir().join("annual_report_saved.pdf");
    fs::write(&saved_dest, dummy_file_bytes).unwrap();
    if let Some(DecryptedContent::File {
        ref mut saved_path, ..
    }) = app.decrypted
    {
        *saved_path = Some(saved_dest.clone());
    }
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "直接打开文件");
    text_center(&output, "在文件夹中显示");
    text_center(&output, "另存为");
    let _ = fs::remove_file(&saved_dest);

    // 5. Clicking "立即销毁 / 隐藏密文" resets the receive state completely
    click(&mut app, &ctx, text_center(&output, "立即销毁 / 隐藏密文"));
    assert!(app.decrypted.is_none());
    assert!(app.receive_file.is_none());
    assert!(app.receive_input.is_empty());
    assert!(app.receive_password.is_empty());
}

#[test]
fn binary_payload_legacy_fallback_never_shows_bare_text() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);

    // Encrypt raw binary data that contains null bytes (not an Envelope JSON)
    let raw_binary = b"PK\x03\x04\x00\x00\x08\x00\x00\x00\x00\x00archive content";
    let password = "fallback-pass";
    let cipher = encrypt_payload(password, raw_binary).unwrap();

    app.tab = Tab::Receive;
    app.receive_input = cipher;
    app.receive_password = password.into();
    app.do_receive();

    // Must be classified as File, NEVER raw lossy text with question-mark diamonds
    match &app.decrypted {
        Some(DecryptedContent::File { bytes, .. }) => {
            assert_eq!(bytes, raw_binary);
        }
        Some(DecryptedContent::Text(_)) => {
            panic!("Binary fallback must NOT decode to bare lossy text");
        }
        None => panic!("Decryption failed"),
    }
}

#[test]
fn password_review_card_and_delivery_guidance() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    app.send_text = "重要机密口令测试".into();
    app.send_password = "my-secret-passphrase".into();
    app.send_confirm = app.send_password.clone();

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);

    app.do_send();
    assert!(!app.generated_cipher.is_empty());
    assert!(!app.generated_show_password);

    // Initial generated output view
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "本次加密口令");
    text_center(&output, "复制口令");
    text_center(&output, "复制分流指引");
    text_center(&output, "显示");

    // Initially, masked password should appear in text shapes (contains bullet dots), not plaintext
    let shapes_text: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                Some(t.galley.job.text.clone())
            } else {
                None
            }
        })
        .collect();
    assert!(shapes_text.iter().any(|t| t.contains('●')));
    assert!(!shapes_text.iter().any(|t| t == "my-secret-passphrase"));

    // Click "显示" to toggle password visibility
    let show_pt = text_center(&output, "显示");
    click(&mut app, &ctx, show_pt);
    assert!(app.generated_show_password);

    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "隐藏");
    let shapes_text_after: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                Some(t.galley.job.text.clone())
            } else {
                None
            }
        })
        .collect();
    assert!(shapes_text_after
        .iter()
        .any(|t| t == "my-secret-passphrase"));

    // Click "复制口令"
    let copy_pass_pt = text_center(&output, "复制口令");
    click(&mut app, &ctx, copy_pass_pt);
    assert!(matches!(&app.toast, Some(t) if t.message.contains("加密口令已复制")));

    // Click "复制分流指引"
    let copy_guide_pt = text_center(&output, "复制分流指引");
    click(&mut app, &ctx, copy_guide_pt);
    assert!(matches!(&app.toast, Some(t) if t.message.contains("密信分流指引已复制")));
    let guidance = app.delivery_guidance(false);
    assert!(guidance.contains(&app.generated_cipher));
    assert!(guidance.contains("分流发送"));

    // Test file mode delivery guidance
    let file_guidance = app.delivery_guidance(true);
    assert!(file_guidance.contains(".seal"));
    assert!(file_guidance.contains("火漆"));
    assert!(file_guidance.contains("分流发送"));
}

#[test]
fn file_decryption_saved_actions_and_system_integration() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    app.tab = Tab::Receive;

    let temp_file = std::env::temp_dir().join("test_saved_result.pdf");
    fs::write(&temp_file, b"sample pdf content").unwrap();

    // 1. When not yet saved (saved_path is None), actions shouldn't show
    app.decrypted = Some(DecryptedContent::File {
        meta: FileMeta {
            name: "test_saved_result".into(),
            extension: "pdf".into(),
            content_type: "application/pdf".into(),
        },
        bytes: b"sample pdf content".to_vec(),
        saved_path: None,
    });

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);

    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "保存文件");
    assert!(!output.shapes.iter().any(|s| {
        if let egui::Shape::Text(t) = &s.shape {
            t.galley.job.text.contains("直接打开文件")
                || t.galley.job.text.contains("在文件夹中显示")
        } else {
            false
        }
    }));

    // 2. When saved_path is Some(temp_file), both actions must be visible
    app.decrypted = Some(DecryptedContent::File {
        meta: FileMeta {
            name: "test_saved_result".into(),
            extension: "pdf".into(),
            content_type: "application/pdf".into(),
        },
        bytes: b"sample pdf content".to_vec(),
        saved_path: Some(temp_file.clone()),
    });

    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "直接打开文件");
    text_center(&output, "在文件夹中显示");
    text_center(&output, "另存为");

    // Native helpers should succeed when file exists
    assert!(ui::reveal_in_file_manager(&temp_file).is_ok());
    assert!(ui::open_file_native(&temp_file).is_ok());
    assert!(app.reveal_saved_file().is_ok());
    assert!(app.open_saved_file().is_ok());

    let _ = fs::remove_file(&temp_file);
    // When file does not exist, helpers return Err
    assert!(ui::reveal_in_file_manager(&temp_file).is_err());
    assert!(ui::open_file_native(&temp_file).is_err());
}

#[test]
fn clipboard_detection_and_auto_fill_flow() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    app.tab = Tab::Receive;

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);

    let sample_cipher = "MX1.abcdef1234567890".to_string();
    app.clipboard_cipher = Some(sample_cipher.clone());

    // When clipboard has cipher and input is empty, capsule is rendered
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "检测到剪贴板中的密信");
    text_center(&output, "一键填入");
    text_center(&output, "忽略");

    // Click "一键填入"
    let fill_pt = text_center(&output, "一键填入");
    click(&mut app, &ctx, fill_pt);

    assert_eq!(app.receive_input, sample_cipher);
    assert!(app.clipboard_cipher.is_none());
    // In the click turn, focus was transferred directly to the password widget in the newly rendered card
    assert!(ctx.memory(|m| m.focused().is_some()));
    assert!(matches!(&app.toast, Some(t) if t.message.contains("已载入剪贴板中的密信")));

    // On next frame, capsule is gone and password decrypt card is rendered
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "口令密信");
    text_center(&output, "解密口令");
    text_center(&output, "解密密信");
    assert!(!output.shapes.iter().any(|s| {
        if let egui::Shape::Text(t) = &s.shape {
            t.galley.job.text.contains("检测到剪贴板中的密信")
        } else {
            false
        }
    }));

    // Test "忽略" action
    app.receive_input.clear();
    app.clipboard_cipher = Some(sample_cipher.clone());
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let ignore_pt = text_center(&output, "忽略");
    click(&mut app, &ctx, ignore_pt);

    assert!(app.clipboard_cipher.is_none());
    assert_eq!(
        app.clipboard_ignored.as_deref(),
        Some(sample_cipher.as_str())
    );
}

#[test]
fn enter_key_triggers_password_decryption_flow() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    app.send_text = "通过回车直接解密的机密消息".into();
    app.send_password = "enter-pass-phrase".into();
    app.send_confirm = app.send_password.clone();
    app.do_send();
    let cipher = app.generated_cipher.clone();

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);

    // Switch to receive tab and paste cipher
    app.tab = Tab::Receive;
    app.receive_input = cipher;
    app.focus_receive_password = true;

    // First render settles focus on the password field
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "解密口令");
    assert!(!app.focus_receive_password); // focus was consumed

    // Enter without password gives a prompt toast
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.decrypted.is_none());
    assert!(matches!(&app.toast, Some(t) if t.message.contains("请输入解密口令")));

    // Fill password and press Enter
    app.receive_password = "enter-pass-phrase".into();
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );

    // Decryption must succeed!
    match &app.decrypted {
        Some(DecryptedContent::Text(text)) => {
            assert_eq!(text, "通过回车直接解密的机密消息");
        }
        _ => panic!("Expected successful text decryption on Enter key"),
    }
}

#[test]
fn test_text_input_expansion_word_count_and_draft_isolation() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    app.send_text = "这是十四个字符的机密内容呀。".into();

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);

    // Initial render in text mode
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "共 14 字");

    // Switch to file mode and select a file
    click(&mut app, &ctx, text_center(&output, "文件"));
    assert!(app.payload_type == PayloadType::File);
    app.send_file = Some(SelectedFile {
        path: PathBuf::from("test.txt"),
        name: "test".into(),
        extension: "txt".into(),
        size: 100,
        bytes: vec![1, 2, 3],
    });

    // Switch back to text mode: both drafts must be preserved!
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    click(&mut app, &ctx, text_center(&output, "文字"));
    assert!(app.payload_type == PayloadType::Text);
    assert_eq!(app.send_text, "这是十四个字符的机密内容呀。");
    assert!(app.send_file.is_some(), "File draft must not be cleared when switching to text");

    // Switch back to file mode: file draft still there
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    click(&mut app, &ctx, text_center(&output, "文件"));
    assert!(app.payload_type == PayloadType::File);
    assert_eq!(app.send_file.as_ref().unwrap().name, "test");
}

#[test]
fn test_password_embedded_eye_and_overwrite_protection() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);

    // Ensure legacy "显示口令" checkbox does not exist in the send tab
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let texts: Vec<String> = output
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                Some(t.galley.job.text.clone())
            } else {
                None
            }
        })
        .collect();
    assert!(!texts.iter().any(|t| t == "显示口令"));

    // When empty, clicking "随机生成并复制" generates immediately without confirmation
    let gen_pt = text_center(&output, "随机生成并复制");
    click(&mut app, &ctx, gen_pt);
    assert!(!app.send_password.is_empty());
    assert!(!app.confirm_overwrite_password);

    // Now set a manual password
    app.send_password = "my-custom-secret-123".into();
    app.send_confirm = "my-custom-secret-123".into();

    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let gen_pt = text_center(&output, "随机生成并复制");
    click(&mut app, &ctx, gen_pt);

    // Overwrite was blocked, confirmation dialog requested
    assert_eq!(app.send_password, "my-custom-secret-123");
    assert!(app.confirm_overwrite_password);

    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "已有手动输入口令，确认覆盖？");
    let cancel_pt = text_center(&output, "取消");
    click(&mut app, &ctx, cancel_pt);

    // User cancelled: password retained, confirmation dismissed
    assert_eq!(app.send_password, "my-custom-secret-123");
    assert!(!app.confirm_overwrite_password);

    // Click again and confirm overwrite
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let gen_pt = text_center(&output, "随机生成并复制");
    click(&mut app, &ctx, gen_pt);
    assert!(app.confirm_overwrite_password);

    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let confirm_pt = text_center(&output, "确认覆盖");
    click(&mut app, &ctx, confirm_pt);

    // Password overwritten with fresh random password
    assert!(!app.confirm_overwrite_password);
    assert_ne!(app.send_password, "my-custom-secret-123");
    assert_eq!(app.send_password, app.send_confirm);
}

#[test]
fn test_contact_row_click_navigation_and_inline_rename() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    app.contacts.push(Contact {
        name: "李四".into(),
        public_key: identity.1.clone(),
        fingerprint: compute_fingerprint(&identity.1),
        created_at: "2025-01-01".into(),
    });

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);
    theme::apply_theme(&ctx, false);

    app.tab = Tab::Identity;

    // First check: In normal state, row displays contact name and "发密信 →"
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let contact_pt = text_center(&output, "李四");

    // Clicking the contact row directly navigates to Send tab in contact mode
    click(&mut app, &ctx, contact_pt);
    assert!(app.tab == Tab::Send);
    assert!(app.send_mode == SendMode::Contact);
    assert_eq!(app.selected_contact_idx, Some(0));

    // Return to Identity tab
    app.tab = Tab::Identity;
    // Enter inline edit mode
    app.editing_contact_idx = Some(0);
    app.editing_contact_name = "李四 (开发经理)".into();

    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    let save_pt = text_center(&output, "保存");
    click(&mut app, &ctx, save_pt);

    // Contact name updated and edit mode exited
    assert_eq!(app.contacts[0].name, "李四 (开发经理)");
    assert_eq!(app.editing_contact_idx, None);
}

#[test]
fn test_font_scale_strictly_constrained() {
    let allowed_sizes: [f32; 5] = [11.0, 12.5, 14.0, 16.0, 20.0];
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);

    for dark in [false, true] {
        theme::apply_theme(&ctx, dark);
        for tab in [Tab::Send, Tab::Receive, Tab::Identity] {
            app.tab = tab;
            let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
            for shape in &output.shapes {
                if let egui::Shape::Text(text) = &shape.shape {
                    let size = text.galley.job.sections.first().map_or(0.0, |s| s.format.font_id.size);
                    if size > 0.0 {
                        assert!(
                            allowed_sizes.iter().any(|&allowed| (allowed - size).abs() < 0.1),
                            "Font size {size} px found in {tab:?}, dark={dark}, not in allowed scale {allowed_sizes:?}: text '{}'",
                            text.galley.job.text
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_global_keyboard_flow() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);

    // Initial tab is Send
    assert_eq!(app.tab, Tab::Send);

    // Cmd + 2 switches to Receive
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Num2,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    assert_eq!(app.tab, Tab::Receive);

    // Ctrl + 3 switches to Identity
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Num3,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }],
    );
    assert_eq!(app.tab, Tab::Identity);

    // Cmd + 1 switches back to Send
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Num1,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    assert_eq!(app.tab, Tab::Send);

    // Prepare message in Send tab
    app.send_text = "全局快捷键测试内容".into();
    app.send_password = "keyboard-secret".into();
    app.send_confirm = "keyboard-secret".into();

    // Cmd + Enter triggers send
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    assert!(!app.generated_cipher.is_empty(), "Cmd+Enter must generate cipher");

    // Esc returns to edit
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.generated_cipher.is_empty(), "Esc must return to edit draft");

    // Switch to receive, input cipher, test Cmd + Enter for decryption
    app.tab = Tab::Receive;
    let envelope = Envelope {
        kind: "text".into(),
        meta: None,
        data: base64::engine::general_purpose::STANDARD.encode(b"decrypted via cmd enter"),
    };
    let packet = serde_json::to_vec(&envelope).unwrap();
    let cipher = encrypt_payload("keyboard-secret", &packet).unwrap();
    app.receive_input = cipher;
    app.receive_password = "keyboard-secret".into();

    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    assert!(app.decrypted.is_some(), "Cmd+Enter must trigger decryption");

    // Esc destroys decrypted content
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.decrypted.is_none(), "Esc must destroy decrypted result");
}

#[test]
fn test_password_shake_and_inline_error_feedback() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);

    app.tab = Tab::Receive;
    let envelope = Envelope {
        kind: "text".into(),
        meta: None,
        data: base64::engine::general_purpose::STANDARD.encode(b"test secret"),
    };
    let packet = serde_json::to_vec(&envelope).unwrap();
    let cipher = encrypt_payload("correct-password", &packet).unwrap();
    app.receive_input = cipher;
    app.receive_password = "wrong-password".into();

    // Trigger receive with wrong password
    app.do_receive();

    assert_eq!(app.password_error_msg.as_deref(), Some("口令不匹配"));
    assert!(app.password_error_at.is_some());
    assert!(matches!(&app.toast, Some(t) if t.message.contains("解密失败")));

    // Rendering should include inline error text
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "口令不匹配");

    // Modifying the password field clears the error
    app.receive_password = "correct-password".into();
    app.password_error_msg = None;
    app.password_error_at = None;
    app.do_receive();
    assert!(app.decrypted.is_some());
    assert!(app.password_error_msg.is_none());
}

#[test]
fn test_privacy_blur_mask_and_destroy_flow() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);

    app.tab = Tab::Receive;
    app.decrypted = Some(DecryptedContent::Text("机密会议纪要".into()));

    // When window is focused, decrypted text is visible
    app.window_focused = true;
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output, "密信已解密");
    text_center(&output, "立即销毁 / 隐藏密文");

    // When window loses focus, privacy mask appears
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::WindowFocused(false)],
    );
    assert!(!app.window_focused);
    let output_blurred = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output_blurred, "隐私保护中 · 内容已遮蔽");
    text_center(&output_blurred, "恢复显示");

    // Clicking "恢复显示" restores visibility
    click(&mut app, &ctx, text_center(&output_blurred, "恢复显示"));
    let output_restored = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output_restored, "密信已解密");

    // Clicking "立即销毁 / 隐藏密文" resets memory completely
    click(&mut app, &ctx, text_center(&output_restored, "立即销毁 / 隐藏密文"));
    assert!(app.decrypted.is_none());
    assert!(app.receive_input.is_empty());
    assert!(app.receive_password.is_empty());
}

#[test]
fn test_identity_card_restructuring_and_qr_modal() {
    let identity = generate_identity().unwrap();
    let mut app = fixture(&identity);
    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);

    app.tab = Tab::Identity;
    let output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);

    // 4-character spaced fingerprint is clearly displayed
    let spaced = format_fingerprint_spaced(&app.my_fingerprint);
    text_center(&output, &spaced);
    text_center(&output, "安全身份指纹");
    text_center(&output, "复制指纹");
    text_center(&output, "查看/保存二维码");

    // Clicking "查看/保存二维码" opens modal dialog
    click(&mut app, &ctx, text_center(&output, "查看/保存二维码"));
    assert!(app.show_qr_dialog);

    let modal_output = render(&mut app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&modal_output, "身份二维码");
    text_center(&modal_output, "保存图片");
    text_center(&modal_output, "关闭");

    // Esc closes the modal dialog
    render(
        &mut app,
        &ctx,
        1040.0,
        800.0,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(!app.show_qr_dialog, "Esc must close QR modal dialog");
}

#[test]
fn test_contact_encryption_and_verified_sender_ui() {
    let (alice_priv, alice_pub) = generate_identity().unwrap();
    let (bob_priv, bob_pub) = generate_identity().unwrap();

    let mut bob_app = App::with_identity(
        (bob_priv, bob_pub.clone()),
        vec![Contact {
            name: "爱丽丝".into(),
            public_key: alice_pub.clone(),
            fingerprint: compute_fingerprint(&alice_pub),
            created_at: "2025-01-01".into(),
        }],
        false,
    );

    let ctx = egui::Context::default();
    theme::configure_fonts(&ctx);

    // Alice encrypts for Bob
    let envelope = Envelope {
        kind: "text".into(),
        meta: None,
        data: base64::engine::general_purpose::STANDARD.encode(b"hello from alice"),
    };
    let packet = serde_json::to_vec(&envelope).unwrap();
    let cipher = encrypt_for(&alice_priv, &bob_pub, &packet).unwrap();
    assert!(cipher.starts_with(IDENTITY_PREFIX));

    bob_app.tab = Tab::Receive;
    bob_app.receive_input = cipher;

    let output_before = render(&mut bob_app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output_before, "这封密信只有本机身份能打开");
    text_center(&output_before, "使用本机身份解密");

    // Click decrypt
    click(&mut bob_app, &ctx, text_center(&output_before, "使用本机身份解密"));

    assert!(bob_app.decrypted.is_some());
    assert_eq!(
        bob_app.verified_sender,
        Some("爱丽丝 · 已验证签名".to_string())
    );

    let output_after = render(&mut bob_app, &ctx, 1040.0, 800.0, vec![]);
    text_center(&output_after, "密信已解密");
    text_center(&output_after, "发件人：爱丽丝 · 已验证签名");
    text_center(&output_after, "复制文字");
}
