use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

use anyhow::Result;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

pub fn init() -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("console.txt")?;

    *LOG_FILE.lock().unwrap() = Some(file);

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("PANIC: {}", info);
        eprintln!("{}", msg);
        if let Ok(mut guard) = LOG_FILE.lock() {
            if let Some(ref mut file) = *guard {
                let _ = writeln!(file, "{}", msg);
                let _ = file.flush();
            }
        }
        default_hook(info);
    }));
    Ok(())
}

pub fn log(msg: &str) {
    println!("{}", msg);
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "{}", msg);
            let _ = file.flush();
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        let s = format!($($arg)*);
        $crate::logger::log(&s);
    }};
}
