# Doxyde Website Structure & Content Plan

## Website Structure

### 1. **Homepage** (`/`)
- Hero section: "Build Content That Matters"
- Key features (Fast, AI-Native, Developer Friendly, Secure)
- Quick start CTA
- Use cases / Who it's for

### 2. **Features** (`/features`)
- **Core Features** (`/features/core`)
  - Component-based content system
  - Version control built-in
  - Hierarchical page structure
  - Draft/publish workflow
  
- **Developer Features** (`/features/developer`)
  - Clean API design
  - Extensible architecture
  - CLI tools
  - Testing framework
  
- **Security** (`/features/security`)
  - Rust memory safety
  - Role-based access control
  - Session management
  - Content sanitization

### 3. **Documentation** (`/docs`)
- **Getting Started** (`/docs/getting-started`)
  - Installation
  - Quick start guide
  - First site setup
  
- **User Guide** (`/docs/user-guide`)
  - Content management
  - Page properties
  - Component templates
  - Image management
  
- **Developer Guide** (`/docs/developer-guide`)
  - Architecture overview
  - API reference
  - Extension development
  - Contributing guide
  
- **CLI Reference** (`/docs/cli`)
  - All commands
  - Examples
  - Configuration

### 4. **Use Cases** (`/use-cases`)
- **Personal Blogs** (`/use-cases/blogs`)
- **Business Websites** (`/use-cases/business`)
- **Documentation Sites** (`/use-cases/documentation`)
- **Developer Portfolios** (`/use-cases/portfolio`)

### 5. **About** (`/about`)
- **Project Story** (`/about/story`)
  - Why Doxyde exists
  - Vision and goals
  - Roadmap
  
- **Team** (`/about/team`)
  - Contributors
  - How to contribute
  
- **License** (`/about/license`)
  - AGPLv3 explanation
  - Commercial licensing

### 6. **Community** (`/community`)
- GitHub repository link
- Issue tracker
- Discussions
- Contributing guide
- Code of conduct

### 7. **Blog** (`/blog`)
- Development updates
- Tutorials
- Case studies
- Technical deep-dives

### Footer Links
- Quick Links: Features, Docs, GitHub
- Resources: CLI Reference, API Docs
- Legal: License, Privacy Policy
- Social: GitHub, Twitter/X

## Key Content Themes

### Homepage Content Focus
- **Hero**: Emphasize Rust performance, AI-readiness, and developer experience
- **Problem/Solution**: Address CMS complexity and performance issues
- **Social Proof**: Showcase performance metrics, test coverage
- **CTA**: "Get Started in 5 Minutes"

### Feature Pages Content
- **Technical specs**: Performance benchmarks, security features
- **Code examples**: Show how easy it is to extend
- **Comparison**: vs traditional CMS platforms
- **Architecture diagrams**: Visual explanations

### Documentation Strategy
- **Progressive disclosure**: Start simple, add complexity
- **Task-oriented**: "How to..." guides
- **Reference**: Complete API documentation
- **Examples**: Real-world use cases

### Marketing Messages
1. **Performance**: "Built with Rust for unmatched speed"
2. **AI-Native**: "Designed for the AI era"
3. **Developer-First**: "Clean APIs, great DX"
4. **Open Source**: "AGPLv3 with commercial options"

## Content Templates

### Feature Page Template
```markdown
# [Feature Name]

## Overview
Brief description of the feature and its benefits.

## How It Works
Technical explanation with diagrams if applicable.

## Code Example
```rust
// Show relevant code snippet
```

## Benefits
- Bullet points of key advantages
- Performance metrics if available
- Comparison with alternatives

## Learn More
- Link to documentation
- Related features
- Tutorials
```

### Use Case Page Template
```markdown
# Doxyde for [Use Case]

## The Challenge
What problems this use case faces.

## The Solution
How Doxyde addresses these challenges.

## Key Features
- Relevant features for this use case
- Specific benefits

## Example Sites
Screenshots or demos if available.

## Get Started
Quick steps to implement this use case.
```

### Documentation Page Template
```markdown
# [Topic]

## Prerequisites
What users need to know before reading.

## Overview
High-level explanation of the topic.

## Step-by-Step Guide
1. First step with code/commands
2. Second step with explanations
3. Continue as needed

## Common Issues
- Problem: Solution
- Problem: Solution

## Next Steps
- Related topics
- Advanced features
```

## Implementation Notes

1. **Navigation**: Keep the main nav simple (Features, Docs, About, Community)
2. **Search**: Implement site-wide search for documentation
3. **Mobile**: Ensure responsive design throughout
4. **Dark Mode**: Support both light and dark themes
5. **Performance**: Static generation where possible
6. **SEO**: Optimize all pages with proper meta tags
7. **Analytics**: Add privacy-respecting analytics
8. **Feedback**: Add feedback widget on documentation pages