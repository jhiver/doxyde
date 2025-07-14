// Doxyde - A modern, AI-native CMS built with Rust
// Copyright (C) 2025 Doxyde Project Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use serde_json::Value;

/// Convert style options JSON to inline CSS styles
pub fn style_options_to_css(style_options: Option<&Value>) -> String {
    let Some(options) = style_options else {
        return String::new();
    };

    let mut styles = Vec::new();

    // Background styles
    if let Some(background) = options.get("background") {
        if let Some(bg_type) = background.get("type").and_then(|t| t.as_str()) {
            match bg_type {
                "color" => {
                    if let Some(color) = background.get("value").and_then(|v| v.as_str()) {
                        styles.push(format!("background-color: {}", color));
                    }
                }
                "gradient" => {
                    if let Some(gradient) = background.get("gradient").and_then(|g| g.as_str()) {
                        styles.push(format!("background: {}", gradient));
                    }
                }
                "image" => {
                    if let Some(image_url) = background.get("image_url").and_then(|u| u.as_str()) {
                        styles.push(format!("background-image: url('{}')", image_url));
                        styles.push("background-size: cover".to_string());
                        styles.push("background-position: center".to_string());
                        styles.push("background-repeat: no-repeat".to_string());
                    }
                }
                _ => {}
            }

            // Opacity
            if let Some(opacity) = background.get("opacity").and_then(|o| o.as_f64()) {
                if opacity < 1.0 {
                    styles.push(format!("opacity: {}", opacity));
                }
            }
        }
    }

    // Spacing styles
    if let Some(spacing) = options.get("spacing") {
        if let Some(padding) = spacing.get("padding").and_then(|p| p.as_str()) {
            styles.push(format!("padding: {}", padding));
        }
        if let Some(margin) = spacing.get("margin").and_then(|m| m.as_str()) {
            styles.push(format!("margin: {}", margin));
        }
    }

    // Layout styles
    if let Some(layout) = options.get("layout") {
        if let Some(max_width) = layout.get("max_width").and_then(|w| w.as_str()) {
            styles.push(format!("max-width: {}", max_width));
        }
        if let Some(alignment) = layout.get("alignment").and_then(|a| a.as_str()) {
            match alignment {
                "center" => styles.push("margin-left: auto; margin-right: auto".to_string()),
                "left" => styles.push("margin-right: auto".to_string()),
                "right" => styles.push("margin-left: auto".to_string()),
                _ => {}
            }
        }
    }

    if styles.is_empty() {
        String::new()
    } else {
        format!(" style=\"{}\"", styles.join("; "))
    }
}

/// Get CSS classes based on style options
pub fn style_options_to_classes(style_options: Option<&Value>) -> Vec<String> {
    let Some(options) = style_options else {
        return vec![];
    };

    let mut classes = vec![];

    // Effects
    if let Some(effects) = options.get("effects") {
        if effects
            .get("shadow")
            .and_then(|s| s.as_bool())
            .unwrap_or(false)
        {
            classes.push("component-shadow".to_string());
        }
        if effects
            .get("rounded")
            .and_then(|r| r.as_bool())
            .unwrap_or(false)
        {
            classes.push("component-rounded".to_string());
        }
        if effects
            .get("bordered")
            .and_then(|b| b.as_bool())
            .unwrap_or(false)
        {
            classes.push("component-bordered".to_string());
        }
    }

    // Background type classes
    if let Some(background) = options.get("background") {
        if let Some(bg_type) = background.get("type").and_then(|t| t.as_str()) {
            match bg_type {
                "image" => classes.push("has-bg-image".to_string()),
                "gradient" => classes.push("has-bg-gradient".to_string()),
                _ => {}
            }
        }
    }

    classes
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_style_options_to_css_empty() {
        assert_eq!(style_options_to_css(None), "");
        assert_eq!(style_options_to_css(Some(&json!({}))), "");
    }

    #[test]
    fn test_style_options_to_css_background_color() {
        let options = json!({
            "background": {
                "type": "color",
                "value": "#ff0000"
            }
        });
        assert_eq!(
            style_options_to_css(Some(&options)),
            " style=\"background-color: #ff0000\""
        );
    }

    #[test]
    fn test_style_options_to_css_background_gradient() {
        let options = json!({
            "background": {
                "type": "gradient",
                "gradient": "linear-gradient(45deg, #ff0000, #00ff00)"
            }
        });
        assert_eq!(
            style_options_to_css(Some(&options)),
            " style=\"background: linear-gradient(45deg, #ff0000, #00ff00)\""
        );
    }

    #[test]
    fn test_style_options_to_css_background_image() {
        let options = json!({
            "background": {
                "type": "image",
                "image_url": "/uploads/hero.jpg"
            }
        });
        let css = style_options_to_css(Some(&options));
        assert!(css.contains("background-image: url('/uploads/hero.jpg')"));
        assert!(css.contains("background-size: cover"));
        assert!(css.contains("background-position: center"));
    }

    #[test]
    fn test_style_options_to_css_spacing() {
        let options = json!({
            "spacing": {
                "padding": "2rem",
                "margin": "1rem auto"
            }
        });
        let css = style_options_to_css(Some(&options));
        assert!(css.contains("padding: 2rem"));
        assert!(css.contains("margin: 1rem auto"));
    }

    #[test]
    fn test_style_options_to_css_layout() {
        let options = json!({
            "layout": {
                "max_width": "1200px",
                "alignment": "center"
            }
        });
        let css = style_options_to_css(Some(&options));
        assert!(css.contains("max-width: 1200px"));
        assert!(css.contains("margin-left: auto; margin-right: auto"));
    }

    #[test]
    fn test_style_options_to_css_combined() {
        let options = json!({
            "background": {
                "type": "color",
                "value": "#f0f0f0",
                "opacity": 0.8
            },
            "spacing": {
                "padding": "2rem"
            }
        });
        let css = style_options_to_css(Some(&options));
        assert!(css.contains("background-color: #f0f0f0"));
        assert!(css.contains("opacity: 0.8"));
        assert!(css.contains("padding: 2rem"));
    }

    #[test]
    fn test_style_options_to_classes_empty() {
        assert_eq!(style_options_to_classes(None), Vec::<String>::new());
        assert_eq!(
            style_options_to_classes(Some(&json!({}))),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_style_options_to_classes_effects() {
        let options = json!({
            "effects": {
                "shadow": true,
                "rounded": true,
                "bordered": false
            }
        });
        let classes = style_options_to_classes(Some(&options));
        assert!(classes.contains(&"component-shadow".to_string()));
        assert!(classes.contains(&"component-rounded".to_string()));
        assert!(!classes.contains(&"component-bordered".to_string()));
    }

    #[test]
    fn test_style_options_to_classes_background() {
        let options = json!({
            "background": {
                "type": "image"
            }
        });
        let classes = style_options_to_classes(Some(&options));
        assert!(classes.contains(&"has-bg-image".to_string()));
    }
}
