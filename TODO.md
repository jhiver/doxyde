=> check action handlers, the yellow menu bar navigation items seem to be inconsistent
=> the create new page should be consistent with what .properties is rendering
=> it should be possible to edit the slug in .properties, or leave blank for default slugification
=> it should be possible to view previous page versions
=> in content.rs, `pub async fn content_handler` is a huge function that needs to be refactored. content handlers should use some kind of registry. Functions should never be this long