//! 密信：双击打开，输入密码，加密/解密一句话。
//! 两台电脑用同一个密码，就能传敏感信息。

mod crypto;

use crypto::{decrypt, encrypt};
use std::io::{self, Write};
use std::process::{Command, Stdio};

fn main() {
    setup_console();
    println!();
    println!("================================");
    println!("            密  信");
    println!("  两台电脑，同一个密码，传一句秘密");
    println!("================================");
    println!();
    println!("  1. 加密  — 把话变成乱码，发给对方");
    println!("  2. 解密  — 把收到的乱码还原");
    println!();

    loop {
        let choice = read_line("请输入 1 或 2：");
        match choice.as_str() {
            "1" => {
                do_encrypt();
                break;
            }
            "2" => {
                do_decrypt();
                break;
            }
            _ => println!("请输入数字 1 或 2。"),
        }
    }

    println!();
    let _ = read_line("按回车退出");
}

fn do_encrypt() {
    println!();
    println!("双方要记住同一个密码。");
    println!("密码请当面说 / 打电话说，不要和乱码发在同一个聊天里。");
    println!();

    let password = read_password_confirmed();
    let plain = loop {
        let s = read_line("要保密的话：");
        if s.is_empty() {
            println!("内容不能为空。");
            continue;
        }
        break s;
    };

    match encrypt(&password, &plain) {
        Ok(token) => {
            println!();
            println!("—— 把下面整行发给对方 ——");
            println!();
            println!("{token}");
            println!();
            if copy_to_clipboard(&token) {
                println!("已复制到剪贴板，去聊天窗口粘贴就行。");
            } else {
                println!("请手动选中上面那一行复制。");
            }
        }
        Err(e) => println!("加密失败：{e}"),
    }
}

fn do_decrypt() {
    println!();
    let password = read_password("密码：");
    if password.is_empty() {
        println!("密码不能为空。");
        return;
    }
    let token = loop {
        let s = read_line("把收到的乱码贴在这里：");
        if s.is_empty() {
            println!("乱码不能为空。");
            continue;
        }
        break s;
    };

    match decrypt(&password, &token) {
        Ok(plain) => {
            println!();
            println!("======= 原文 =======");
            println!("{plain}");
            println!("====================");
        }
        Err(e) => println!("{e}"),
    }
}

fn read_password_confirmed() -> String {
    loop {
        let a = read_password("设一个密码：");
        if a.is_empty() {
            println!("密码不能为空。");
            continue;
        }
        let b = read_password("再输一遍：");
        if a == b {
            return a;
        }
        println!("两次不一样，重新设。");
    }
}

fn read_password(prompt: &str) -> String {
    match rpassword::prompt_password(prompt) {
        Ok(s) => s,
        Err(_) => read_line(prompt),
    }
}

fn read_line(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut s = String::new();
    if io::stdin().read_line(&mut s).is_err() {
        return String::new();
    }
    s.trim().to_string()
}

fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        pipe_to("pbcopy", text.as_bytes())
    }
    #[cfg(target_os = "windows")]
    {
        let mut utf16 = vec![0xFF, 0xFE];
        for u in text.encode_utf16() {
            utf16.extend_from_slice(&u.to_le_bytes());
        }
        pipe_to("clip", &utf16)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = text;
        false
    }
}

fn pipe_to(cmd: &str, bytes: &[u8]) -> bool {
    let mut child = match Command::new(cmd).stdin(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(_) => return false,
    };
    if let Some(stdin) = child.stdin.as_mut() {
        if stdin.write_all(bytes).is_err() {
            return false;
        }
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

fn setup_console() {
    #[cfg(windows)]
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn SetConsoleOutputCP(code: u32) -> i32;
    fn SetConsoleCP(code: u32) -> i32;
}
