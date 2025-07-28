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

pub mod discovery;
pub mod handlers;
pub mod oauth;

pub use discovery::{
    oauth_authorization_server_metadata, oauth_protected_resource_mcp_metadata,
    oauth_protected_resource_metadata, options_handler,
};
pub use handlers::{handle_http, handle_sse};
pub use oauth::{
    authorize, authorize_consent, create_token, list_tokens, oauth_options, register_client,
    revoke_token, token,
};
