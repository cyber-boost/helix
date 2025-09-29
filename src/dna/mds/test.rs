#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from([
            "helix",
            "compile",
            "test.hlx",
            "-O3",
            "--compress",
        ]);
        assert!(cli.is_ok());
    }
}