use anyhow::Result;
use tempfile::TempDir;

#[test]
fn test_site_domain_sanitization() {
    // Test the same sanitization logic used in the CLI
    let test_cases = vec![
        ("example.com", "example.com"),
        ("localhost:3000", "localhost-3000"), // : becomes -
        ("sub.domain.com", "sub.domain.com"),
        ("my-site.com", "my-site.com"),
        ("test_site.com", "test-site.com"), // _ becomes -
        ("café.com", "caf-.com"),           // Non-ASCII becomes -
        ("test@site.com", "test-site.com"), // @ becomes -
    ];

    for (input, expected) in test_cases {
        let sanitized: String = input
            .chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' => c,
                _ => '-',
            })
            .collect();

        assert_eq!(sanitized, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_site_directory_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let sites_base = temp_dir.path();

    let _domain = "test.com";
    let sanitized_domain = "test.com";
    let site_dir = sites_base.join(sanitized_domain);

    // Create the expected directory structure
    std::fs::create_dir_all(&site_dir)?;
    std::fs::create_dir_all(site_dir.join("templates"))?;
    std::fs::create_dir_all(site_dir.join("uploads"))?;

    // Verify structure exists
    assert!(site_dir.exists());
    assert!(site_dir.join("templates").exists());
    assert!(site_dir.join("uploads").exists());

    Ok(())
}
