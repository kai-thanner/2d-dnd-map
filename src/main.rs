use clap::{Args, Parser, Subcommand};
use dnd_2d_map::config::{Climate, Config};
use dnd_2d_map::edit::{Edit, GameState, PoiKind};
use dnd_2d_map::gen;
use dnd_2d_map::render::ascii::AsciiRenderer;
use dnd_2d_map::render::Renderer;
use dnd_2d_map::rng;
use dnd_2d_map::save;
use dnd_2d_map::world::{Biome, SettlementKind};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "dnd-2d-map", version, about = "Prozeduraler 2D-Karten-Generator für das DnD-Universum")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Generate(GenerateArgs),
    Show {
        file: PathBuf,
        #[arg(long)]
        legend: bool,
    },
    Edit(EditArgs),
}

#[derive(Args)]
struct GenerateArgs {
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long, default_value_t = 120)]
    width: usize,
    #[arg(long, default_value_t = 60)]
    height: usize,
    #[arg(long, default_value_t = 0.42)]
    sea_level: f32,
    #[arg(long, default_value_t = 0.6)]
    moisture: f32,
    #[arg(long, default_value_t = 0.5)]
    settlements: f32,
    #[arg(long, default_value_t = 28.0)]
    south_temp: f32,
    #[arg(long, default_value_t = -6.0)]
    north_temp: f32,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long)]
    print: bool,
}

#[derive(Args)]
struct EditArgs {
    file: PathBuf,
    #[command(subcommand)]
    cmd: EditCmd,
}

#[derive(Subcommand)]
enum EditCmd {
    AddDungeon { x: usize, y: usize },
    RemoveDungeon { x: usize, y: usize },
    AddSettlement { x: usize, y: usize, kind: String, name: Option<String> },
    DestroySettlement { x: usize, y: usize },
    Burn { x: usize, y: usize, radius: u32 },
    Drain { x: usize, y: usize, radius: u32 },
    Flood { x: usize, y: usize, radius: u32 },
    Road { x1: usize, y1: usize, x2: usize, y2: usize },
    Poi { x: usize, y: usize, kind: String },
    SetBiome { x: usize, y: usize, biome: String },
}

fn parse_kind(s: &str) -> Result<SettlementKind, String> {
    match s.to_ascii_lowercase().as_str() {
        "city" | "stadt" => Ok(SettlementKind::City),
        "village" | "dorf" => Ok(SettlementKind::Village),
        other => Err(format!("unbekannter Siedlungstyp: {other} (city|village)")),
    }
}

fn parse_poi_kind(s: &str) -> Result<PoiKind, String> {
    match s.to_ascii_lowercase().as_str() {
        "dungeon" => Ok(PoiKind::Dungeon),
        "inn" | "gasthaus" => Ok(PoiKind::Inn),
        "merchant" | "haendler" => Ok(PoiKind::Merchant),
        other => Err(format!(
            "unbekannter POI-Typ: {other} (dungeon|inn|merchant)"
        )),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Generate(a) => {
            let cfg = Config {
                seed: a.seed,
                width: a.width,
                height: a.height,
                sea_level: a.sea_level,
                moisture_scale: a.moisture,
                settlement_density: a.settlements,
                climate: Climate {
                    south_temp: a.south_temp,
                    north_temp: a.north_temp,
                    ..Climate::default()
                },
            };
            let world = gen::generate(&cfg);
            let state = GameState::new(world);
            if let Some(out) = &a.out {
                save::save(out, &cfg, &state)?;
                println!("Karte gespeichert: {} (Seed {})", out.display(), cfg.seed);
            }
            if a.print || a.out.is_none() {
                print!("{}", AsciiRenderer.render(&state.world));
                print!("{}", AsciiRenderer::legend());
            }
        }
        Commands::Show { file, legend } => {
            let (_cfg, state) = save::load(&file)?;
            print!("{}", AsciiRenderer.render(&state.world));
            if legend {
                print!("{}", AsciiRenderer::legend());
            }
            println!("Revision: {}", state.revision);
        }
        Commands::Edit(a) => {
            let (cfg, mut state) = save::load(&a.file)?;
            let edit = match a.cmd {
                EditCmd::AddDungeon { x, y } => Edit::AddDungeon { x, y },
                EditCmd::RemoveDungeon { x, y } => Edit::RemoveDungeon { x, y },
                EditCmd::AddSettlement { x, y, kind, name } => Edit::AddSettlement {
                    x,
                    y,
                    kind: parse_kind(&kind)?,
                    name,
                },
                EditCmd::DestroySettlement { x, y } => Edit::DestroySettlement { x, y },
                EditCmd::Burn { x, y, radius } => Edit::BurnForest { x, y, radius },
                EditCmd::Drain { x, y, radius } => Edit::DrainLake { x, y, radius },
                EditCmd::Flood { x, y, radius } => Edit::Flood { x, y, radius },
                EditCmd::Road { x1, y1, x2, y2 } => Edit::AddRoad {
                    from: (x1, y1),
                    to: (x2, y2),
                },
                EditCmd::Poi { x, y, kind } => Edit::AddPoi {
                    x,
                    y,
                    kind: parse_poi_kind(&kind)?,
                },
                EditCmd::SetBiome { x, y, biome } => Edit::SetBiome {
                    x,
                    y,
                    biome: Biome::from_str(&biome)?,
                },
            };
            let mut rng = rng::from_seed(cfg.seed ^ 0xED17_5EED);
            state.apply(edit, &mut rng);
            save::save(&a.file, &cfg, &state)?;
            println!(
                "Edit angewendet, Revision {} – gespeichert: {}",
                state.revision,
                a.file.display()
            );
            print!("{}", AsciiRenderer.render(&state.world));
        }
    }
    Ok(())
}
