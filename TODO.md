=> in content.rs, `pub async fn content_handler` is a huge function that needs to be refactored. content handlers should use some kind of registry. Functions should never be this long...
=> www.<domain> doesn't redirect to <domain>
=> prettier 404, 500 pages, with customisable templates
=> it should be possible to view previous page versions
=> it should be possible to add page and content handlers from the interface, and the template names should not be hardcoded, should be extracted from disk and database
=> find a way to include AI tools in the mix, so that it's possible to click a button to generate content or tell what to do differently