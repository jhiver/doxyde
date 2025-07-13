use crate::models::component::Component;
use crate::models::component_trait::{escape_html, extract_text, ComponentRenderer};

pub struct ImageComponent {
    pub id: Option<i64>,
    pub src: String,
    pub alt: String,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub display_width: Option<String>,
    pub display_height: Option<String>,
}

impl ImageComponent {
    pub fn from_component(component: &Component) -> Self {
        // Check if this is the new format with slug
        if let Some(slug) = component.content.get("slug").and_then(|s| s.as_str()) {
            let format = component
                .content
                .get("format")
                .and_then(|f| f.as_str())
                .unwrap_or("jpg");
            let src = format!("/{}.{}", slug, format);
            let alt = component
                .content
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or_else(|| {
                    component
                        .content
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                })
                .to_string();
            let width = component
                .content
                .get("width")
                .and_then(|w| w.as_u64())
                .map(|w| w as u32);
            let height = component
                .content
                .get("height")
                .and_then(|h| h.as_u64())
                .map(|h| h as u32);

            let display_width = component
                .content
                .get("display_width")
                .and_then(|w| w.as_str())
                .filter(|w| !w.is_empty())
                .map(|w| w.to_string());
            let display_height = component
                .content
                .get("display_height")
                .and_then(|h| h.as_str())
                .filter(|h| !h.is_empty())
                .map(|h| h.to_string());

            Self {
                id: component.id,
                src,
                alt,
                title: component.title.clone().or_else(|| {
                    component
                        .content
                        .get("title")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                }),
                width,
                height,
                display_width,
                display_height,
            }
        } else {
            // Old format
            Self {
                id: component.id,
                src: extract_text(&component.content, "src"),
                alt: extract_text(&component.content, "alt"),
                title: component.title.clone(),
                width: None,
                height: None,
                display_width: None,
                display_height: None,
            }
        }
    }
}

impl ComponentRenderer for ImageComponent {
    fn render(&self, template: &str) -> String {
        // Build img tag with width/height if available
        let mut img_attrs = vec![
            format!(r#"src="{}""#, escape_html(&self.src)),
            format!(r#"alt="{}""#, escape_html(&self.alt)),
        ];

        // Use original dimensions as HTML attributes for aspect ratio
        if let Some(width) = self.width {
            img_attrs.push(format!(r#"width="{}""#, width));
        }
        if let Some(height) = self.height {
            img_attrs.push(format!(r#"height="{}""#, height));
        }

        img_attrs.push(r#"loading="lazy""#.to_string());

        // Build style attribute with display dimensions
        let mut style_parts = vec![];
        
        if let Some(display_width) = &self.display_width {
            style_parts.push(format!("width: {}", display_width));
        } else {
            style_parts.push("max-width: 100%".to_string());
        }
        
        if let Some(display_height) = &self.display_height {
            style_parts.push(format!("height: {}", display_height));
        } else {
            style_parts.push("height: auto".to_string());
        }
        
        let style_attr = if !style_parts.is_empty() {
            format!(r#" style="{}""#, style_parts.join("; "))
        } else {
            String::new()
        };

        let img_tag = format!(
            r#"<img {}{}>"#,
            img_attrs.join(" "),
            style_attr
        );

        match template {
            "default" => {
                format!(r#"<div class="image-component">{}</div>"#, img_tag)
            }
            "figure" => {
                format!(
                    r#"<figure class="image-component figure">
    {}
    {}
</figure>"#,
                    img_tag,
                    self.title
                        .as_ref()
                        .map(|t| format!(r#"<figcaption>{}</figcaption>"#, escape_html(t)))
                        .unwrap_or_default()
                )
            }
            "hero" => {
                format!(
                    r#"<div class="image-component hero" style="width: 100%; overflow: hidden;">
    <img src="{}" alt="{}" style="width: 100%; height: auto; display: block;">
</div>"#,
                    escape_html(&self.src),
                    escape_html(&self.alt)
                )
            }
            "gallery" => {
                format!(
                    r#"<div class="image-component gallery-item">
    <a href="{}" data-lightbox="gallery" data-title="{}">
        {}
    </a>
    {}
</div>"#,
                    escape_html(&self.src),
                    escape_html(self.title.as_ref().unwrap_or(&self.alt)),
                    img_tag,
                    self.title
                        .as_ref()
                        .map(|t| format!(
                            r#"<div class="gallery-caption">{}</div>"#,
                            escape_html(t)
                        ))
                        .unwrap_or_default()
                )
            }
            "thumbnail" => {
                format!(
                    r#"<div class="image-component thumbnail" style="width: 150px; height: 150px; overflow: hidden; display: inline-block;">
    <img src="{}" alt="{}" style="width: 100%; height: 100%; object-fit: cover;">
</div>"#,
                    escape_html(&self.src),
                    escape_html(&self.alt)
                )
            }
            "responsive" => {
                // For responsive images, we could generate multiple sizes
                // For now, just use the regular image with responsive classes
                format!(
                    r#"<picture class="image-component responsive">
    {}
</picture>"#,
                    img_tag
                )
            }
            "hidden" => {
                // Hidden template - renders nothing but keeps the component data
                String::new()
            }
            _ => self.render("default"),
        }
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec![
            "default",
            "figure",
            "hero",
            "gallery",
            "thumbnail",
            "responsive",
            "hidden",
        ]
    }
}
