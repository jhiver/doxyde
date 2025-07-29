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

pub mod action;
pub mod auth;
pub mod delete_page;
pub mod edit;
pub mod image_serve;
pub mod image_upload;
pub mod move_page;
pub mod pages;
pub mod properties;
pub mod reorder;
pub mod shared;
pub mod sites;

pub use action::handle_action;
pub use auth::{login, login_form, logout};
pub use delete_page::{delete_page_handler, do_delete_page_handler};
pub use edit::{
    add_component_handler, create_page_handler, delete_component_handler, discard_draft_handler,
    edit_page_content_handler, move_component_handler, new_page_handler, publish_draft_handler,
    save_draft_handler, update_component_handler,
};
pub use image_serve::{image_preview_handler, serve_image_handler};
pub use image_upload::{
    upload_component_image_handler, upload_image_ajax_handler, upload_image_handler,
};
pub use move_page::{do_move_page_handler, move_page_handler};
pub use properties::{page_properties_handler, update_page_properties_handler};
pub use reorder::{reorder_page_handler, update_page_order_handler};
