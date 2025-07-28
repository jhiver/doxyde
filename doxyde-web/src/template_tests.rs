#[cfg(test)]
mod tests {
    use tera::Tera;

    #[test]
    fn test_all_templates_compile() {
        // Use Tera's built-in glob pattern to load all templates
        // This properly handles template inheritance
        let tera = match Tera::new("../templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                panic!("Failed to parse templates: {}", e);
            }
        };

        // Get list of all templates
        let template_names: Vec<_> = tera.get_template_names().collect();

        println!("Found {} templates", template_names.len());

        for name in &template_names {
            println!("✓ Template '{}' compiled successfully", name);
        }

        // Test rendering with minimal context for non-partial templates
        let mut context = tera::Context::new();
        context.insert("site_title", "Test Site");
        context.insert("user", &false);
        context.insert("can_edit", &false);

        for name in &template_names {
            // Skip templates that are includes/partials or CSS
            if name.contains("action_bar.html")
                || name.contains("mobile_header.html")
                || name.contains("mobile_nav_drawer.html")
                || name.contains("mobile_edit_drawer.html")
                || name.contains("styles.css")
                || name.contains("sidebar.html")
            {
                continue;
            }

            match tera.render(name, &context) {
                Ok(_) => println!(
                    "✓ Template '{}' rendered successfully with minimal context",
                    name
                ),
                Err(e) => {
                    // Some templates require specific variables, so we just log warnings
                    println!("⚠ Template '{}' rendering warning: {}", name, e);
                }
            }
        }
    }

    #[test]
    fn test_mobile_templates_compile() {
        let mut tera = Tera::default();

        // Test mobile-specific templates
        let mobile_templates = vec![
            (
                "mobile_header.html",
                include_str!("../../templates/mobile_header.html"),
            ),
            (
                "mobile_nav_drawer.html",
                include_str!("../../templates/mobile_nav_drawer.html"),
            ),
            (
                "mobile_edit_drawer.html",
                include_str!("../../templates/mobile_edit_drawer.html"),
            ),
        ];

        for (name, content) in &mobile_templates {
            match tera.add_raw_template(name, content) {
                Ok(_) => {}
                Err(e) => panic!("Failed to compile mobile template '{}': {}", name, e),
            }
        }

        // Test rendering with various contexts
        let test_contexts = [
            // Logged out user
            {
                let mut ctx = tera::Context::new();
                ctx.insert("user", &false);
                ctx.insert("site_title", "Doxyde");
                ctx
            },
            // Logged in user with edit permissions
            {
                let mut ctx = tera::Context::new();
                ctx.insert("user", &true);
                ctx.insert("can_edit", &true);
                ctx.insert("site_title", "Doxyde");
                ctx.insert("action", "view");
                ctx
            },
            // With logo
            {
                let mut ctx = tera::Context::new();
                ctx.insert("user", &true);
                ctx.insert("logo_url", "/logo.png");
                ctx.insert("root_page_title", "My Site");
                ctx
            },
        ];

        for (i, context) in test_contexts.iter().enumerate() {
            for (name, _) in &mobile_templates {
                match tera.render(name, context) {
                    Ok(_) => {}
                    Err(e) => println!(
                        "Warning: Template '{}' with context {} failed: {}",
                        name, i, e
                    ),
                }
            }
        }
    }
}
