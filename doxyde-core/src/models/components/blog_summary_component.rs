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

use crate::models::component::Component;
use crate::models::component_trait::{escape_html, ComponentRenderer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogSummaryConfig {
    pub parent_page_id: i64,
    pub display_title: Option<String>,
    pub item_count: i32,
    pub order_by: String,
    pub show_descriptions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogSummaryPage {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub created_at: String,
    pub url: String,
}

pub struct BlogSummaryComponent {
    pub id: Option<i64>,
    pub config: BlogSummaryConfig,
    pub pages: Vec<BlogSummaryPage>,
    pub title: Option<String>,
}

impl BlogSummaryComponent {
    pub fn from_component(component: &Component) -> Self {
        // Parse the config from component content
        let config: BlogSummaryConfig = serde_json::from_value(component.content.clone())
            .unwrap_or(BlogSummaryConfig {
                parent_page_id: 0,
                display_title: None,
                item_count: 5,
                order_by: "created_at_desc".to_string(),
                show_descriptions: true,
            });

        // Extract pages if they've been injected
        let pages: Vec<BlogSummaryPage> = component
            .content
            .get("pages")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Self {
            id: component.id,
            config,
            pages,
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for BlogSummaryComponent {
    fn render(&self, template: &str) -> String {
        // If no pages are loaded, show a placeholder
        if self.pages.is_empty() {
            return format!(
                r#"<div class="blog-summary empty">
                    <p>No articles to display</p>
                </div>"#
            );
        }

        match template {
            "cards" => self.render_cards(),
            "list" => self.render_list(),
            "definition" => self.render_definition(),
            "compact" => self.render_compact(),
            "timeline" => self.render_timeline(),
            "featured" => self.render_featured(),
            _ => self.render_cards(), // Default to cards
        }
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec![
            "cards",
            "list",
            "definition",
            "compact",
            "timeline",
            "featured",
        ]
    }
}

impl BlogSummaryComponent {
    fn render_cards(&self) -> String {
        let mut html = String::from(r#"<div class="blog-summary cards">"#);

        if let Some(ref title) = self.config.display_title {
            html.push_str(&format!(
                r#"<h2 class="summary-title">{}</h2>"#,
                escape_html(title)
            ));
        }

        html.push_str(r#"<div class="summary-grid">"#);

        for page in &self.pages {
            html.push_str(&format!(
                r#"<div class="summary-card">
                    <h3><a href="{}">{}</a></h3>"#,
                escape_html(&page.url),
                escape_html(&page.title)
            ));

            if self.config.show_descriptions {
                if let Some(ref desc) = page.description {
                    html.push_str(&format!(
                        r#"<p class="summary-description">{}</p>"#,
                        escape_html(desc)
                    ));
                }
            }

            html.push_str(&format!(
                r#"<time class="summary-date">{}</time>
                </div>"#,
                escape_html(&page.created_at)
            ));
        }

        html.push_str("</div></div>");
        html
    }

    fn render_list(&self) -> String {
        let mut html = String::from(r#"<div class="blog-summary list">"#);

        if let Some(ref title) = self.config.display_title {
            html.push_str(&format!(
                r#"<h2 class="summary-title">{}</h2>"#,
                escape_html(title)
            ));
        }

        html.push_str(r#"<ul class="summary-list">"#);

        for page in &self.pages {
            html.push_str(&format!(
                r#"<li class="summary-item">
                    <h3><a href="{}">{}</a></h3>"#,
                escape_html(&page.url),
                escape_html(&page.title)
            ));

            if self.config.show_descriptions {
                if let Some(ref desc) = page.description {
                    html.push_str(&format!(
                        r#"<p class="summary-description">{}</p>"#,
                        escape_html(desc)
                    ));
                }
            }

            html.push_str("</li>");
        }

        html.push_str("</ul></div>");
        html
    }

    fn render_definition(&self) -> String {
        let mut html = String::from(r#"<div class="blog-summary definition">"#);

        if let Some(ref title) = self.config.display_title {
            html.push_str(&format!(
                r#"<h2 class="summary-title">{}</h2>"#,
                escape_html(title)
            ));
        }

        html.push_str(r#"<dl class="summary-list">"#);

        for page in &self.pages {
            html.push_str(&format!(
                r#"<dt><a href="{}">{}</a></dt>"#,
                escape_html(&page.url),
                escape_html(&page.title)
            ));

            if self.config.show_descriptions {
                if let Some(ref desc) = page.description {
                    html.push_str(&format!(r#"<dd>{}</dd>"#, escape_html(desc)));
                }
            }
        }

        html.push_str("</dl></div>");
        html
    }

    fn render_compact(&self) -> String {
        let mut html = String::from(r#"<div class="blog-summary compact">"#);

        if let Some(ref title) = self.config.display_title {
            html.push_str(&format!(
                r#"<h2 class="summary-title">{}</h2>"#,
                escape_html(title)
            ));
        }

        html.push_str(r#"<ul class="summary-compact-list">"#);

        for page in &self.pages {
            html.push_str(&format!(
                r#"<li><a href="{}">{}</a></li>"#,
                escape_html(&page.url),
                escape_html(&page.title)
            ));
        }

        html.push_str("</ul></div>");
        html
    }

    fn render_timeline(&self) -> String {
        let mut html = String::from(r#"<div class="blog-summary timeline">"#);

        if let Some(ref title) = self.config.display_title {
            html.push_str(&format!(
                r#"<h2 class="summary-title">{}</h2>"#,
                escape_html(title)
            ));
        }

        html.push_str(r#"<div class="timeline-items">"#);

        for page in &self.pages {
            html.push_str(&format!(
                r#"<div class="timeline-item">
                    <time class="timeline-date">{}</time>
                    <div class="timeline-content">
                        <h3><a href="{}">{}</a></h3>"#,
                escape_html(&page.created_at),
                escape_html(&page.url),
                escape_html(&page.title)
            ));

            if self.config.show_descriptions {
                if let Some(ref desc) = page.description {
                    html.push_str(&format!(
                        r#"<p class="summary-description">{}</p>"#,
                        escape_html(desc)
                    ));
                }
            }

            html.push_str("</div></div>");
        }

        html.push_str("</div></div>");
        html
    }

    fn render_featured(&self) -> String {
        let mut html = String::from(r#"<div class="blog-summary featured">"#);

        if let Some(ref title) = self.config.display_title {
            html.push_str(&format!(
                r#"<h2 class="summary-title">{}</h2>"#,
                escape_html(title)
            ));
        }

        html.push_str(r#"<div class="featured-items">"#);

        if let Some(featured) = self.pages.first() {
            html.push_str(&format!(
                r#"<div class="featured-primary">
                    <h3><a href="{}">{}</a></h3>"#,
                escape_html(&featured.url),
                escape_html(&featured.title)
            ));

            if self.config.show_descriptions {
                if let Some(ref desc) = featured.description {
                    html.push_str(&format!(
                        r#"<p class="featured-description">{}</p>"#,
                        escape_html(desc)
                    ));
                }
            }

            html.push_str(&format!(
                r#"<time class="featured-date">{}</time>
                </div>"#,
                escape_html(&featured.created_at)
            ));
        }

        if self.pages.len() > 1 {
            html.push_str(r#"<div class="secondary-items">"#);

            for page in self.pages.iter().skip(1) {
                html.push_str(&format!(
                    r#"<div class="secondary-item">
                        <h4><a href="{}">{}</a></h4>
                        <time>{}</time>
                    </div>"#,
                    escape_html(&page.url),
                    escape_html(&page.title),
                    escape_html(&page.created_at)
                ));
            }

            html.push_str("</div>");
        }

        html.push_str("</div></div>");
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_blog_summary_from_component() {
        let component = Component {
            id: Some(1),
            page_version_id: 1,
            component_type: "blog_summary".to_string(),
            position: 0,
            content: serde_json::json!({
                "parent_page_id": 10,
                "display_title": "Latest Posts",
                "item_count": 3,
                "order_by": "created_at_desc",
                "show_descriptions": true
            }),
            title: None,
            template: "cards".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let blog_summary = BlogSummaryComponent::from_component(&component);
        assert_eq!(blog_summary.config.parent_page_id, 10);
        assert_eq!(
            blog_summary.config.display_title,
            Some("Latest Posts".to_string())
        );
        assert_eq!(blog_summary.config.item_count, 3);
    }

    #[test]
    fn test_render_empty() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: Some("Latest Posts".to_string()),
                item_count: 5,
                order_by: "created_at_desc".to_string(),
                show_descriptions: true,
            },
            pages: vec![],
            title: None,
        };

        let html = component.render("cards");
        assert!(html.contains("No articles to display"));
    }

    #[test]
    fn test_available_templates() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: None,
                item_count: 5,
                order_by: "created_at_desc".to_string(),
                show_descriptions: true,
            },
            pages: vec![],
            title: None,
        };

        let templates = component.get_available_templates();
        assert_eq!(templates.len(), 6);
        assert!(templates.contains(&"cards"));
        assert!(templates.contains(&"list"));
        assert!(templates.contains(&"timeline"));
    }

    #[test]
    fn test_render_cards_with_pages() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: Some("Latest Posts".to_string()),
                item_count: 5,
                order_by: "created_at_desc".to_string(),
                show_descriptions: true,
            },
            pages: vec![
                BlogSummaryPage {
                    id: 1,
                    slug: "post-1".to_string(),
                    title: "First Post".to_string(),
                    description: Some("This is the first post".to_string()),
                    created_at: "2025-01-01T12:00:00Z".to_string(),
                    url: "/blog/post-1".to_string(),
                },
                BlogSummaryPage {
                    id: 2,
                    slug: "post-2".to_string(),
                    title: "Second Post".to_string(),
                    description: None,
                    created_at: "2025-01-02T12:00:00Z".to_string(),
                    url: "/blog/post-2".to_string(),
                },
            ],
            title: None,
        };

        let html = component.render("cards");
        assert!(html.contains("blog-summary cards"));
        assert!(html.contains("Latest Posts"));
        assert!(html.contains("First Post"));
        assert!(html.contains("This is the first post"));
        assert!(html.contains("Second Post"));
        assert!(html.contains("/blog/post-1"));
        assert!(html.contains("/blog/post-2"));
    }

    #[test]
    fn test_render_list_without_descriptions() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: Some("Articles".to_string()),
                item_count: 5,
                order_by: "created_at_desc".to_string(),
                show_descriptions: false,
            },
            pages: vec![BlogSummaryPage {
                id: 1,
                slug: "post-1".to_string(),
                title: "First Post".to_string(),
                description: Some("This description should not appear".to_string()),
                created_at: "2025-01-01T12:00:00Z".to_string(),
                url: "/blog/post-1".to_string(),
            }],
            title: None,
        };

        let html = component.render("list");
        assert!(html.contains("blog-summary list"));
        assert!(html.contains("Articles"));
        assert!(html.contains("First Post"));
        assert!(!html.contains("This description should not appear"));
    }

    #[test]
    fn test_render_timeline() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: Some("Post Timeline".to_string()),
                item_count: 2,
                order_by: "created_at_asc".to_string(),
                show_descriptions: true,
            },
            pages: vec![
                BlogSummaryPage {
                    id: 1,
                    slug: "old-post".to_string(),
                    title: "Old Post".to_string(),
                    description: Some("An older post".to_string()),
                    created_at: "2024-12-01T12:00:00Z".to_string(),
                    url: "/blog/old-post".to_string(),
                },
                BlogSummaryPage {
                    id: 2,
                    slug: "new-post".to_string(),
                    title: "New Post".to_string(),
                    description: Some("A newer post".to_string()),
                    created_at: "2025-01-01T12:00:00Z".to_string(),
                    url: "/blog/new-post".to_string(),
                },
            ],
            title: None,
        };

        let html = component.render("timeline");
        assert!(html.contains("blog-summary timeline"));
        assert!(html.contains("Post Timeline"));
        assert!(html.contains("timeline-items"));
        assert!(html.contains("2024-12-01T12:00:00Z"));
        assert!(html.contains("2025-01-01T12:00:00Z"));
    }

    #[test]
    fn test_render_compact() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: None,
                item_count: 3,
                order_by: "title_asc".to_string(),
                show_descriptions: false,
            },
            pages: vec![
                BlogSummaryPage {
                    id: 1,
                    slug: "aaa".to_string(),
                    title: "AAA Post".to_string(),
                    description: None,
                    created_at: "2025-01-01T12:00:00Z".to_string(),
                    url: "/blog/aaa".to_string(),
                },
                BlogSummaryPage {
                    id: 2,
                    slug: "bbb".to_string(),
                    title: "BBB Post".to_string(),
                    description: None,
                    created_at: "2025-01-02T12:00:00Z".to_string(),
                    url: "/blog/bbb".to_string(),
                },
            ],
            title: None,
        };

        let html = component.render("compact");
        assert!(html.contains("blog-summary compact"));
        assert!(!html.contains("summary-title")); // No title
        assert!(html.contains("compact-list"));
        assert!(html.contains("AAA Post"));
        assert!(html.contains("BBB Post"));
    }

    #[test]
    fn test_render_featured_single_item() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: Some("Featured Post".to_string()),
                item_count: 1,
                order_by: "created_at_desc".to_string(),
                show_descriptions: true,
            },
            pages: vec![BlogSummaryPage {
                id: 1,
                slug: "featured".to_string(),
                title: "The Featured Post".to_string(),
                description: Some("This is the featured post".to_string()),
                created_at: "2025-01-01T12:00:00Z".to_string(),
                url: "/blog/featured".to_string(),
            }],
            title: None,
        };

        let html = component.render("featured");
        assert!(html.contains("blog-summary featured"));
        assert!(html.contains("Featured Post"));
        assert!(html.contains("featured-primary"));
        assert!(html.contains("The Featured Post"));
        assert!(html.contains("This is the featured post"));
        assert!(!html.contains("secondary-items")); // Only one item
    }

    #[test]
    fn test_render_featured_multiple_items() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: Some("Featured Posts".to_string()),
                item_count: 3,
                order_by: "created_at_desc".to_string(),
                show_descriptions: true,
            },
            pages: vec![
                BlogSummaryPage {
                    id: 1,
                    slug: "main".to_string(),
                    title: "Main Feature".to_string(),
                    description: Some("Main featured post".to_string()),
                    created_at: "2025-01-03T12:00:00Z".to_string(),
                    url: "/blog/main".to_string(),
                },
                BlogSummaryPage {
                    id: 2,
                    slug: "second".to_string(),
                    title: "Second Feature".to_string(),
                    description: Some("Second post".to_string()),
                    created_at: "2025-01-02T12:00:00Z".to_string(),
                    url: "/blog/second".to_string(),
                },
                BlogSummaryPage {
                    id: 3,
                    slug: "third".to_string(),
                    title: "Third Feature".to_string(),
                    description: Some("Third post".to_string()),
                    created_at: "2025-01-01T12:00:00Z".to_string(),
                    url: "/blog/third".to_string(),
                },
            ],
            title: None,
        };

        let html = component.render("featured");
        assert!(html.contains("blog-summary featured"));
        assert!(html.contains("featured-primary"));
        assert!(html.contains("Main Feature"));
        assert!(html.contains("Main featured post"));
        assert!(html.contains("secondary-items"));
        assert!(html.contains("Second Feature"));
        assert!(html.contains("Third Feature"));
    }

    #[test]
    fn test_render_with_html_escaping() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: Some("<script>alert('test')</script>".to_string()),
                item_count: 1,
                order_by: "created_at_desc".to_string(),
                show_descriptions: true,
            },
            pages: vec![BlogSummaryPage {
                id: 1,
                slug: "xss-test".to_string(),
                title: "Title with <b>HTML</b>".to_string(),
                description: Some("Description with <script>bad code</script>".to_string()),
                created_at: "2025-01-01T12:00:00Z".to_string(),
                url: "/blog/xss-test".to_string(),
            }],
            title: None,
        };

        let html = component.render("cards");
        // Check that HTML is escaped
        assert!(html.contains("&lt;script&gt;alert(&#39;test&#39;)&lt;/script&gt;"));
        assert!(html.contains("Title with &lt;b&gt;HTML&lt;/b&gt;"));
        assert!(html.contains("Description with &lt;script&gt;bad code&lt;/script&gt;"));
    }

    #[test]
    fn test_render_unknown_template_fallback() {
        let component = BlogSummaryComponent {
            id: Some(1),
            config: BlogSummaryConfig {
                parent_page_id: 10,
                display_title: Some("Test".to_string()),
                item_count: 1,
                order_by: "created_at_desc".to_string(),
                show_descriptions: true,
            },
            pages: vec![BlogSummaryPage {
                id: 1,
                slug: "test".to_string(),
                title: "Test Post".to_string(),
                description: None,
                created_at: "2025-01-01T12:00:00Z".to_string(),
                url: "/test".to_string(),
            }],
            title: None,
        };

        // Unknown template should fall back to cards
        let html = component.render("unknown_template");
        assert!(html.contains("blog-summary cards"));
    }
}
