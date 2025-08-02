fn extract_base_domain(domain: &str) -> String {
    // Remove port if present
    let domain_no_port = domain.split(':').next().unwrap_or(domain);
    
    // Split by dots
    let parts: Vec<&str> = domain_no_port.split('.').collect();
    
    // If we have at least 2 parts, take the last 2 as the base domain
    // This handles most common cases like example.com, example.org, etc.
    if parts.len() >= 2 {
        let base = format\!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]);
        base
    } else {
        // For single-part domains (like "localhost"), return as-is
        domain_no_port.to_string()
    }
}

fn main() {
    let test_domain = "test@example.com";
    println\!("Input: {}", test_domain);
    println\!("Base domain: {}", extract_base_domain(test_domain));
}
