use clap::Args;
use std::path::PathBuf;
use crate::mds::templates;

#[derive(Args)]
pub struct TemplatesArgs {
    /// Name of the template to output (e.g., "minimal", "support")
    #[arg(short, long)]
    name: Option<String>,

    /// Output file path (defaults to stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

pub fn run(args: TemplatesArgs) -> anyhow::Result<()> {
    let templates = templates::get_embedded_templates();
    let template_content = if let Some(ref name) = args.name {
        match templates.iter().find(|(n, _)| n == name) {
            Some((_, content)) => content,
            None => {
                eprintln!("Template '{}' not found. Available templates:", name);
                for (n, _) in templates {
                    eprintln!("  - {}", n);
                }
                anyhow::bail!("Template '{}' not found", name);
            }
        }
    } else {
        // If no name is provided, list available templates
        println!("Available templates:");
        for (n, _) in templates {
            println!("  - {}", n);
        }
        return Ok(());
    };

    match args.output {
        Some(output_path) => {
            std::fs::write(&output_path, template_content)?;
            println!("Template written to {}", output_path.display());
        }
        None => {
            println!("{}", template_content);
        }
    }

    Ok(())
}
