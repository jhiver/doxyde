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

use anyhow::{Context, Result};
use std::path::Path;

use crate::autoreload_templates::TemplateEngine;

pub fn init_templates(templates_dir: &str, development_mode: bool) -> Result<TemplateEngine> {
    // Create templates directory if it doesn't exist
    std::fs::create_dir_all(templates_dir).context("Failed to create templates directory")?;

    // Create default templates if they don't exist
    create_default_templates(templates_dir)?;

    // Create template engine
    let template_engine = TemplateEngine::new(templates_dir, development_mode)?;

    Ok(template_engine)
}

fn create_default_templates(templates_dir: &str) -> Result<()> {
    let base_dir = Path::new(templates_dir);

    // Create base template
    let base_template = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}{{ site_title | default(value="Doxyde") }}{% endblock %}</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            color: #333;
        }
        nav {
            border-bottom: 1px solid #eee;
            padding-bottom: 10px;
            margin-bottom: 20px;
        }
        nav a {
            margin-right: 15px;
            text-decoration: none;
            color: #0066cc;
        }
        nav a:hover {
            text-decoration: underline;
        }
        .auth-info {
            float: right;
            font-size: 0.9em;
        }
        footer {
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #eee;
            font-size: 0.9em;
            color: #666;
        }
    </style>
    {% block head %}{% endblock %}
</head>
<body>
    <nav>
        <a href="/">Home</a>
        {% if user %}
            <span class="auth-info">
                {{ user.username }} | 
                <a href="/.logout">Logout</a>
            </span>
        {% else %}
            <span class="auth-info">
                <a href="/.login">Login</a>
            </span>
        {% endif %}
    </nav>

    <main>
        {% block content %}{% endblock %}
    </main>

    <footer>
        <p>Powered by Doxyde</p>
    </footer>
</body>
</html>"#;

    let base_path = base_dir.join("base.html");
    if !base_path.exists() {
        std::fs::write(&base_path, base_template).context("Failed to create base template")?;
    }

    // Create login template
    let login_template = r#"{% extends "base.html" %}

{% block title %}Login - {{ super() }}{% endblock %}

{% block content %}
<h1>Login</h1>

{% if error %}
<p style="color: red;">{{ error }}</p>
{% endif %}

<form method="post" action="/.login">
    <div style="margin-bottom: 15px;">
        <label for="username">Username or Email:</label><br>
        <input type="text" id="username" name="username" required style="width: 300px; padding: 5px;">
    </div>
    
    <div style="margin-bottom: 15px;">
        <label for="password">Password:</label><br>
        <input type="password" id="password" name="password" required style="width: 300px; padding: 5px;">
    </div>
    
    <div>
        <button type="submit" style="padding: 5px 20px;">Login</button>
    </div>
</form>
{% endblock %}"#;

    let login_path = base_dir.join("login.html");
    if !login_path.exists() {
        std::fs::write(&login_path, login_template).context("Failed to create login template")?;
    }

    // Create page template
    let page_template = r#"{% extends "base.html" %}

{% block title %}{{ page.title }} - {{ super() }}{% endblock %}

{% block content %}
<article>
    <h1>{{ page.title }}</h1>
    
    {% if breadcrumbs %}
    <nav aria-label="breadcrumb">
        {% for crumb in breadcrumbs %}
            {% if not loop.last %}
                <a href="{{ crumb.url }}">{{ crumb.title }}</a> /
            {% else %}
                {{ crumb.title }}
            {% endif %}
        {% endfor %}
    </nav>
    {% endif %}

    <div class="content">
        {% if user %}
        <div style="float: right; margin-bottom: 10px;">
            <a href="{{ request_path | default(value="/") }}/.edit" style="text-decoration: none; color: #0066cc;">✏️ Edit Page</a>
        </div>
        <div style="clear: both;"></div>
        {% endif %}
        
        {% for component in components %}
            {% if component.component_type == "text" %}
                <div class="text-component">
                    {{ component.content.text | safe }}
                </div>
            {% elif component.component_type == "image" %}
                <div class="image-component">
                    <img src="/{{ component.content.slug }}.{{ component.content.format }}" 
                         alt="{{ component.content.alt_text | default(value='') }}" 
                         style="max-width: 100%;">
                </div>
            {% elif component.component_type == "code" %}
                <pre><code>{{ component.content.code }}</code></pre>
            {% else %}
                <div class="custom-component">
                    {{ component.content | json_encode }}
                </div>
            {% endif %}
        {% endfor %}
        
        {% if components|length == 0 %}
            <p style="color: #666; font-style: italic;">This page has no content yet.</p>
        {% endif %}
    </div>

    {% if children %}
    <div class="child-pages">
        <h2>Pages</h2>
        <ul>
        {% for child in children %}
            <li><a href="{{ child.url }}">{{ child.title }}</a></li>
        {% endfor %}
        </ul>
    </div>
    {% endif %}
</article>
{% endblock %}"#;

    let page_path = base_dir.join("page.html");
    if !page_path.exists() {
        std::fs::write(&page_path, page_template).context("Failed to create page template")?;
    }

    // Create error template
    let error_template = r#"{% extends "base.html" %}

{% block title %}Error - {{ super() }}{% endblock %}

{% block content %}
<h1>{{ error_title | default(value="Error") }}</h1>
<p>{{ error_message | default(value="An error occurred") }}</p>
<p><a href="/">Return to homepage</a></p>
{% endblock %}"#;

    let error_path = base_dir.join("error.html");
    if !error_path.exists() {
        std::fs::write(&error_path, error_template).context("Failed to create error template")?;
    }

    Ok(())
}
