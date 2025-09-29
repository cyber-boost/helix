use clap::{Args, Subcommand, Parser};
use std::path::PathBuf;
use crate::mds::migrate;

#[derive(Parser, Debug)]
pub struct MigrateArgs {
    #[command(subcommand)]
    pub command: MigrateCommands,
}

#[derive(Subcommand, Debug)]
pub enum MigrateCommands {
    /// Migrate a file from one format to another
    File(FileArgs),
    /// Migrate a directory of files
    Dir(DirArgs),
}

#[derive(Args, Debug)]
pub struct FileArgs {
    /// Input file to migrate
    #[arg(short, long)]
    pub input: PathBuf,
    /// Output file (optional)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Enable verbose output
    #[arg(long, default_value_t = false)]
    pub verbose: bool,
}

#[derive(Args, Debug)]
pub struct DirArgs {
    /// Input directory to migrate
    #[arg(short, long)]
    pub input_dir: PathBuf,
    /// Output directory (optional)
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,
    /// File extension filter (e.g. json, toml, yaml)
    #[arg(short, long)]
    pub extension: Option<String>,
    /// Enable verbose output
    #[arg(long, default_value_t = false)]
    pub verbose: bool,
}

pub async fn run(args: MigrateArgs) -> anyhow::Result<()> {
    match args.command {
        MigrateCommands::File(file_args) => {
            let migrator = migrate::Migrator::new().verbose(file_args.verbose);
            migrator.migrate_file(&file_args.input, file_args.output.as_ref()).map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;
            Ok(())
        }
        MigrateCommands::Dir(dir_args) => {
            let migrator = migrate::Migrator::new().verbose(dir_args.verbose);
            let input_dir = &dir_args.input_dir;
            let output_dir = dir_args.output_dir.as_ref();
            let ext_filter = dir_args.extension.as_deref();
            for entry in std::fs::read_dir(input_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = ext_filter {
                        if path.extension().and_then(|e| e.to_str()) != Some(ext) {
                            continue;
                        }
                    }
                    let output_path = output_dir.map(|od| od.join(path.file_name().unwrap()));
                    migrator.migrate_file(&path, output_path.as_ref())?;
                }
            }
            Ok(())
        }
    }
}