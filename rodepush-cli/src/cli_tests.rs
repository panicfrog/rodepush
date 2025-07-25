//! Tests for CLI command parsing and validation

#[cfg(test)]
mod tests {
    use clap::Parser;
    use std::path::PathBuf;
    
    // Import the CLI structures from main.rs
    use crate::{Cli, Commands, AssetActions};
    
    #[test]
    fn test_cli_parsing_no_args() {
        let args = vec!["rodepush"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        assert!(cli.command.is_none());
        assert!(!cli.verbose);
        assert!(cli.config.is_none());
    }
    
    #[test]
    fn test_cli_parsing_verbose_flag() {
        let args = vec!["rodepush", "--verbose"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        assert!(cli.command.is_none());
        assert!(cli.verbose);
        assert!(cli.config.is_none());
    }
    
    #[test]
    fn test_cli_parsing_config_flag() {
        let args = vec!["rodepush", "--config", "/path/to/config.toml"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        assert!(cli.command.is_none());
        assert!(!cli.verbose);
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
    }
    
    #[test]
    fn test_cli_parsing_build_command() {
        let args = vec![
            "rodepush", 
            "build", 
            "--project-dir", "/path/to/project",
            "--assets-dir", "/path/to/assets",
            "--platform", "android"
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Build { project_dir, assets_dir, platform, .. }) => {
                assert_eq!(project_dir, Some(PathBuf::from("/path/to/project")));
                assert_eq!(assets_dir, Some(PathBuf::from("/path/to/assets")));
                assert_eq!(platform, "android");
            }
            _ => panic!("Expected Build command"),
        }
    }
    
    #[test]
    fn test_cli_parsing_upload_command() {
        let args = vec![
            "rodepush", 
            "upload", 
            "--bundle-path", "/path/to/bundle.js",
            "--assets-dir", "/path/to/assets",
            "--server-url", "https://test.rodepush.com"
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Upload { bundle_path, assets_dir, server_url, .. }) => {
                assert_eq!(bundle_path, Some(PathBuf::from("/path/to/bundle.js")));
                assert_eq!(assets_dir, Some(PathBuf::from("/path/to/assets")));
                assert_eq!(server_url, Some("https://test.rodepush.com".to_string()));
            }
            _ => panic!("Expected Upload command"),
        }
    }
    
    #[test]
    fn test_cli_parsing_deploy_command() {
        let args = vec![
            "rodepush", 
            "deploy", 
            "--app-id", "my-app",
            "--environment", "staging"
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Deploy { app_id, environment }) => {
                assert_eq!(app_id, Some("my-app".to_string()));
                assert_eq!(environment, Some("staging".to_string()));
            }
            _ => panic!("Expected Deploy command"),
        }
    }
    
    #[test]
    fn test_cli_parsing_assets_create_command() {
        let args = vec![
            "rodepush", 
            "assets", 
            "create", 
            "/path/to/assets",
            "--output", "/path/to/output.json"
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Assets { action }) => {
                match action {
                    AssetActions::Create { assets_dir, output } => {
                        assert_eq!(assets_dir, PathBuf::from("/path/to/assets"));
                        assert_eq!(output, Some(PathBuf::from("/path/to/output.json")));
                    }
                    _ => panic!("Expected Create action"),
                }
            }
            _ => panic!("Expected Assets command"),
        }
    }
    
    #[test]
    fn test_cli_parsing_assets_diff_command() {
        let args = vec![
            "rodepush", 
            "assets", 
            "diff", 
            "/path/to/old.json",
            "/path/to/new.json",
            "--output", "/path/to/diff.json"
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Assets { action }) => {
                match action {
                    AssetActions::Diff { old_collection, new_collection, output } => {
                        assert_eq!(old_collection, PathBuf::from("/path/to/old.json"));
                        assert_eq!(new_collection, PathBuf::from("/path/to/new.json"));
                        assert_eq!(output, Some(PathBuf::from("/path/to/diff.json")));
                    }
                    _ => panic!("Expected Diff action"),
                }
            }
            _ => panic!("Expected Assets command"),
        }
    }
    
    #[test]
    fn test_cli_parsing_assets_compress_command() {
        let args = vec![
            "rodepush", 
            "assets", 
            "compress", 
            "/path/to/collection.json",
            "--output", "/path/to/compressed.bin"
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Assets { action }) => {
                match action {
                    AssetActions::Compress { collection, output } => {
                        assert_eq!(collection, PathBuf::from("/path/to/collection.json"));
                        assert_eq!(output, Some(PathBuf::from("/path/to/compressed.bin")));
                    }
                    _ => panic!("Expected Compress action"),
                }
            }
            _ => panic!("Expected Assets command"),
        }
    }
    
    #[test]
    fn test_cli_parsing_assets_decompress_command() {
        let args = vec![
            "rodepush", 
            "assets", 
            "decompress", 
            "/path/to/compressed.bin",
            "--output-dir", "/path/to/output"
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Assets { action }) => {
                match action {
                    AssetActions::Decompress { compressed_collection, output_dir } => {
                        assert_eq!(compressed_collection, PathBuf::from("/path/to/compressed.bin"));
                        assert_eq!(output_dir, Some(PathBuf::from("/path/to/output")));
                    }
                    _ => panic!("Expected Decompress action"),
                }
            }
            _ => panic!("Expected Assets command"),
        }
    }
}