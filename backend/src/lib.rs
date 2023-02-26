pub mod session_state;
pub mod routes;

use oauth2::basic::BasicClient;
use secrecy::Secret;

pub struct YogaAppData {
    pub oauth_client: BasicClient,
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub host: String,
    pub oauth_redirect_url: String,
    pub port: String,
}
