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

    #[test]
    fn test_result_card_guest_capacity_rendering() {
        let tera = match Tera::new("../templates/**/*.html") {
            Ok(t) => t,
            Err(e) => panic!("Failed to parse templates: {}", e),
        };

        let mut labels = std::collections::HashMap::new();
        labels.insert("booking.nights", "nuits");
        labels.insert("booking.for", "pour");
        labels.insert("booking.guests", "Voyageurs");
        labels.insert("booking.guest_one", "Voyageur");
        labels.insert("booking.max", "max :");

        // Test case 1: capacity > searched guests (e.g. 4 capacity, 2 guests)
        let mut ctx1 = tera::Context::new();
        ctx1.insert("q_guests", &2);
        ctx1.insert("q_adults", &2);
        ctx1.insert("q_children", &0);
        ctx1.insert("q_infants", &0);
        ctx1.insert("utm_qs", "");
        ctx1.insert("labels", &labels);
        let r1 = serde_json::json!({
            "is_multi_stay": false,
            "name": "Panoramic Suite",
            "nights": 5,
            "person_capacity": 4,
            "legs": [{"listing_id": 1, "check_in": "2026-10-01", "check_out": "2026-10-06"}]
        });
        ctx1.insert("r", &r1);

        let rendered1 = tera.render("booking/_result_card.html", &ctx1).unwrap();
        assert!(rendered1.contains("5 nuits · pour 2 voyageurs (max : 4)"));

        // Test case 2: capacity == searched guests (e.g. 2 capacity, 2 guests)
        let mut ctx2 = ctx1.clone();
        let r2 = serde_json::json!({
            "is_multi_stay": false,
            "name": "Cozy Studio",
            "nights": 5,
            "person_capacity": 2,
            "legs": [{"listing_id": 2, "check_in": "2026-10-01", "check_out": "2026-10-06"}]
        });
        ctx2.insert("r", &r2);
        let rendered2 = tera.render("booking/_result_card.html", &ctx2).unwrap();
        assert!(rendered2.contains("5 nuits · pour 2 voyageurs"));
        assert!(!rendered2.contains("(max :"));

        // Test case 3: 1 guest, capacity 4
        let mut ctx3 = ctx1.clone();
        ctx3.insert("q_guests", &1);
        ctx3.insert("q_adults", &1);
        let rendered3 = tera.render("booking/_result_card.html", &ctx3).unwrap();
        assert!(rendered3.contains("5 nuits · pour 1 voyageur (max : 4)"));
    }
}
