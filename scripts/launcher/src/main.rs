use std::env;

struct LaunchConfig {
    name: String,
    symbol: String,
    supply: u64,
    decimals: u8,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        println!("Usage: meme-launcher <name> <symbol> <supply> <decimals>");
        std::process::exit(1);
    }

    let config = LaunchConfig {
        name: args[1].clone(),
        symbol: args[2].clone(),
        supply: args[3].parse().expect("Invalid supply"),
        decimals: args[4].parse().expect("Invalid decimals"),
    };

    assert!(!config.name.trim().is_empty(), "Name cannot be empty");
    assert!(!config.symbol.trim().is_empty(), "Symbol cannot be empty");
    assert!(config.supply > 0, "Supply must be greater than zero");
    assert!(config.decimals <= 18, "Decimals must be 0-18");

    println!("=== MEME LAB LAUNCHER ===");
    println!("Name:     {}", config.name);
    println!("Symbol:   {}", config.symbol);
    println!("Supply:   {}", config.supply);
    println!("Decimals: {}", config.decimals);
    println!("Status:   CONFIG GREEN");
}
