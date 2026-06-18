use std::fs;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{self, Command};

const HIPFIRE_ARWAKY_VERSION: &str = "0.1.0";
const HIPFIRE_ARWAKY_DIR_NAME: &str = ".hipfire-arwaky";

fn hipfire_arwaky_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(HIPFIRE_ARWAKY_DIR_NAME)
}

fn config_path() -> PathBuf {
    hipfire_arwaky_dir().join("config.json")
}

fn models_dir() -> PathBuf {
    hipfire_arwaky_dir().join("models")
}

fn upstream_hipfire_bin() -> Option<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let bin = PathBuf::from(home).join(".hipfire").join("bin").join("hipfire");
    if bin.exists() {
        Some(bin)
    } else {
        None
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct HipfireArwakyConfig {
    kv_cache: Option<String>,
    kv_adaptive: Option<String>,
    flash_mode: Option<String>,
    default_model: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    repeat_penalty: Option<f32>,
    max_tokens: Option<u32>,
    max_seq: Option<usize>,
    thinking: Option<String>,
    max_think_tokens: Option<usize>,
    host: Option<String>,
    port: Option<u16>,
    dflash_mode: Option<String>,
}

fn load_config() -> HipfireArwakyConfig {
    let path = config_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HipfireArwakyConfig::default()
    }
}

fn save_config(cfg: &HipfireArwakyConfig) {
    let dir = hipfire_arwaky_dir();
    fs::create_dir_all(&dir).expect("create config dir");
    let content = serde_json::to_string_pretty(cfg).expect("serialize config");
    fs::write(config_path(), content).expect("write config");
}

fn print_usage() {
    println!("hipfire-arwaky v{} — Qwen3.5 on RDNA2 (gfx1030/1031)", HIPFIRE_ARWAKY_VERSION);
    println!();
    println!("Usage: hipfire-arwaky <command> [options]");
    println!();
    println!("Native commands (standalone, uses ~/.hipfire-arwaky/):");
    println!("  run <model.hfq> [prompt]   Interactive REPL or single prompt");
    println!("  chat <model.hfq> [prompt]  Alias for run");
    println!("  config                     Configuration editor (interactive TUI)");
    println!("  config get <key>           Get config value");
    println!("  config set <key> <value>   Set config value");
    println!("  config list                List all config");
    println!("  list [-r]                  List downloaded models in ~/.hipfire-arwaky/models/");
    println!("  version                    Print version info");
    println!("  help                       Show this help");
    println!();
    println!("Delegated commands (require upstream hipfire CLI at ~/.hipfire/bin/hipfire):");
    println!("  serve [host] [port] [-d]   Start daemon server (OpenAI-compatible API)");
    println!("  stop                       Stop daemon server");
    println!("  pull <model>               Download a model (from hipfire registry)");
    println!("  quantize <hf-id|dir>       Quantize model to HFQ");
    println!();
    println!("Config directory: {}", hipfire_arwaky_dir().display());
    println!("Config file:      {}", config_path().display());
}

fn exec_native(bin: &str, cmd_args: &[String]) {
    let exe = std::env::current_exe().expect("current_exe");
    let sub = exe.parent().unwrap().join(bin);
    if !sub.exists() {
        eprintln!("hipfire-arwaky: {} not found (expected at {})", bin, sub.display());
        process::exit(1);
    }
    let err = Command::new(&sub).args(cmd_args).exec();
    eprintln!("hipfire-arwaky: exec {} failed: {}", bin, err);
    process::exit(1);
}

fn delegate_upstream(cmd: &str, args: &[String]) {
    let upstream = match upstream_hipfire_bin() {
        Some(p) => p,
        None => {
            eprintln!("hipfire-arwaky: upstream hipfire CLI not found at ~/.hipfire/bin/hipfire");
            eprintln!("Install upstream hipfire first, or use native commands: run, chat, config, list");
            process::exit(1);
        }
    };

    // Use upstream's config directory by default
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let mut full_args = vec![upstream.to_string_lossy().to_string(), cmd.to_string()];
    full_args.extend(args.iter().cloned());

    let err = Command::new(&full_args[0]).args(&full_args[1..])
        .env("HOME", &home)
        .exec();
    eprintln!("hipfire-arwaky: exec upstream {} failed: {}", upstream.display(), err);
    process::exit(1);
}

fn cmd_config(args: &[String]) {
    let mut cfg = load_config();

    match args.get(0).map(|s| s.as_str()) {
        Some("get") => {
            let key = args.get(1).expect("config get <key>");
            let value = match key.as_str() {
                "kv_cache" => cfg.kv_cache,
                "kv_adaptive" => cfg.kv_adaptive,
                "flash_mode" => cfg.flash_mode,
                "default_model" => cfg.default_model,
                "temperature" => cfg.temperature.map(|v| v.to_string()),
                "top_p" => cfg.top_p.map(|v| v.to_string()),
                "repeat_penalty" => cfg.repeat_penalty.map(|v| v.to_string()),
                "max_tokens" => cfg.max_tokens.map(|v| v.to_string()),
                "max_seq" => cfg.max_seq.map(|v| v.to_string()),
                "thinking" => cfg.thinking,
                "max_think_tokens" => cfg.max_think_tokens.map(|v| v.to_string()),
                "host" => cfg.host,
                "port" => cfg.port.map(|v| v.to_string()),
                "dflash_mode" => cfg.dflash_mode,
                _ => {
                    eprintln!("Unknown config key: {}", key);
                    process::exit(1);
                }
            };
            if let Some(v) = value {
                println!("{}", v);
            } else {
                println!("<not set>");
            }
        }
        Some("set") => {
            let key = args.get(1).expect("config set <key> <value>");
            let value = args.get(2).expect("config set <key> <value>");
            match key.as_str() {
                "kv_cache" => cfg.kv_cache = Some(value.clone()),
                "kv_adaptive" => cfg.kv_adaptive = Some(value.clone()),
                "flash_mode" => cfg.flash_mode = Some(value.clone()),
                "default_model" => cfg.default_model = Some(value.clone()),
                "temperature" => cfg.temperature = Some(value.parse().expect("temperature must be float")),
                "top_p" => cfg.top_p = Some(value.parse().expect("top_p must be float")),
                "repeat_penalty" => cfg.repeat_penalty = Some(value.parse().expect("repeat_penalty must be float")),
                "max_tokens" => cfg.max_tokens = Some(value.parse().expect("max_tokens must be integer")),
                "max_seq" => cfg.max_seq = Some(value.parse().expect("max_seq must be integer")),
                "thinking" => cfg.thinking = Some(value.clone()),
                "max_think_tokens" => cfg.max_think_tokens = Some(value.parse().expect("max_think_tokens must be integer")),
                "host" => cfg.host = Some(value.clone()),
                "port" => cfg.port = Some(value.parse().expect("port must be integer")),
                "dflash_mode" => cfg.dflash_mode = Some(value.clone()),
                _ => {
                    eprintln!("Unknown config key: {}", key);
                    process::exit(1);
                }
            }
            save_config(&cfg);
            println!("Set {} = {}", key, value);
        }
        Some("list") => {
            let json = serde_json::to_string_pretty(&cfg).unwrap();
            println!("{}", json);
        }
        Some("tui") => {
            exec_native("hipfire-arwaky-tui", &[]);
        }
        None => {
            exec_native("hipfire-arwaky-tui", &[]);
        }
        Some(_) => {
            eprintln!("Usage: hipfire-arwaky config [get|set|list|tui]");
            process::exit(1);
        }
    }
}

fn cmd_list(args: &[String]) {
    let show_remote = args.iter().any(|a| a == "-r" || a == "--remote");
    let models = models_dir();

    if models.exists() {
        for entry in fs::read_dir(&models).expect("read models dir") {
            let entry = entry.expect("dir entry");
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".hfq") || name.ends_with(".mq4") || name.ends_with(".mq6") || name.ends_with(".gguf") {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                println!("{} ({:.1} MB)", name, size as f64 / 1e6);
            }
        }
    } else {
        println!("No models found in ~/.hipfire-arwaky/models/");
    }

    if show_remote {
        println!("\nRemote models (from registry):");
        eprintln!("  (not implemented yet, requires upstream hipfire)");
        process::exit(1);
    }
}

fn cmd_pull(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: hipfire-arwaky pull <model>");
        process::exit(1);
    }
    delegate_upstream("pull", args);
}

fn cmd_quantize(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: hipfire-arwaky quantize <hf-id|dir>");
        process::exit(1);
    }
    delegate_upstream("quantize", args);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let cmd = &args[1];
    let rest: &[String] = &args[2..];

    // Ensure config directory exists
    fs::create_dir_all(hipfire_arwaky_dir()).expect("create config dir");
    fs::create_dir_all(models_dir()).expect("create models dir");

    match cmd.as_str() {
        "run" => exec_native("hipfire-arwaky-run", rest),
        "chat" => exec_native("hipfire-arwaky-run", rest),
        "serve" => delegate_upstream("serve", rest),
        "stop" => delegate_upstream("stop", rest),
        "config" => cmd_config(rest),
        "list" | "ls" => cmd_list(rest),
        "pull" => cmd_pull(rest),
        "quantize" => cmd_quantize(rest),
        "tui" => exec_native("hipfire-arwaky-tui", rest),
        "version" | "--version" | "-v" => {
            println!("hipfire-arwaky {}", HIPFIRE_ARWAKY_VERSION);
            println!("  Qwen3.5 on AMD RDNA2 (gfx1030/1031)");
            println!("  Config dir: {}", hipfire_arwaky_dir().display());
        }
        "help" | "--help" | "-h" => print_usage(),
        _ if cmd.ends_with(".hfq") || cmd.contains('/') => {
            exec_native("hipfire-arwaky-run", &args[1..]);
        }
        _ => {
            eprintln!("hipfire-arwaky: unknown command '{}'", cmd);
            eprintln!("Run 'hipfire-arwaky help' for usage.");
            process::exit(1);
        }
    }
}