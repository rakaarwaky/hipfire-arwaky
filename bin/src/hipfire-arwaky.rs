use std::os::unix::process::CommandExt;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let cmd = &args[1];
    let rest: &[String] = &args[2..];

    match cmd.as_str() {
        // Native: run inference via hipfire-arwaky-run
        "run" | "chat" => exec_native("hipfire-arwaky-run", rest),

        // Native: daemon via hipfire-arwaky-daemon
        "serve" => exec_native("hipfire-arwaky-daemon", rest),
        "stop" => exec_native("hipfire-arwaky-daemon", &["stop".to_string()]),

        // TUI config editor
        "tui" => exec_native("hipfire-arwaky-tui", rest),

        // Version
        "version" | "--version" | "-v" => {
            println!("hipfire-arwaky 0.1.0");
            println!("  Qwen3.5 on AMD RDNA2 (gfx1030/1031)");
            println!("  Fork of https://github.com/Kaden-Schutt/hipfire");
        }

        // Delegate all other commands to upstream hipfire CLI
        "list" | "ls" | "pull" | "ps" | "rm" | "config" | "diag"
        | "bench" | "update" | "profile" | "quantize" | "sidecar-gen" => {
            delegate_upstream(cmd, rest);
        }

        // Help
        "help" | "--help" | "-h" => print_usage(),

        // Direct model path shortcut
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

fn exec_native(bin: &str, cmd_args: &[String]) {
    let exe = std::env::current_exe().expect("current_exe");
    let sub = exe.parent().unwrap().join(bin);
    if !sub.exists() {
        eprintln!("hipfire-arwaky: {} not found (expected at {})", bin, sub.display());
        process::exit(1);
    }
    let err = process::Command::new(&sub).args(cmd_args).exec();
    eprintln!("hipfire-arwaky: exec {} failed: {}", bin, err);
    process::exit(1);
}

fn delegate_upstream(cmd: &str, cmd_args: &[String]) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let upstream = std::path::Path::new(&home).join(".hipfire").join("bin").join("hipfire");
    if !upstream.exists() {
        eprintln!("hipfire-arwaky: upstream hipfire CLI not found at {}", upstream.display());
        eprintln!("Install hipfire or use one of the native commands: run, serve, help");
        process::exit(1);
    }
    let mut full_args = vec![upstream.to_string_lossy().to_string(), cmd.to_string()];
    full_args.extend(cmd_args.iter().cloned());
    let err = process::Command::new(&full_args[0]).args(&full_args[1..])
        .env("HOME", &home)
        .exec();
    eprintln!("hipfire-arwaky: exec upstream {} failed: {}", upstream.display(), err);
    process::exit(1);
}

fn print_usage() {
    println!("hipfire-arwaky v0.1.0 — Qwen3.5 on RDNA2 (gfx1030/1031)");
    println!();
    println!("Usage: hipfire-arwaky <command> [options]");
    println!();
    println!("Native commands (run without upstream hipfire):");
    println!("  run|chat <model.hfq>  Interactive REPL");
    println!("  serve [host:port]     Start daemon server");
    println!("  stop                  Stop daemon server");
    println!("  tui                   Terminal UI config editor");
    println!("  version               Print version info");
    println!();
    println!("Delegated commands (require upstream hipfire CLI):");
    println!("  list|ls               List downloaded models");
    println!("  pull <model>          Download a model");
    println!("  ps                    List running daemon processes");
    println!("  rm <model>            Remove a model");
    println!("  config                Configuration editor (upstream)");
    println!("  diag                  System diagnostics");
    println!("  bench                 Run benchmarks");
    println!("  update                Update hipfire");
    println!("  help                  Show this help");
}
