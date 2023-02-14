use actix_web::{
    cookie::{self, Key},
    get,
    http::header::ContentType,
    web, App, HttpResponse, HttpServer, Responder,
};

use actix_session::{
    config::PersistentSession, storage::CookieSessionStore, Session, SessionMiddleware,
};
use backend::session_state::TypedSession;
use dotenv;
use secrecy::{ExposeSecret, Secret};
use tracing_actix_web::TracingLogger;

struct YogaAppData {
    // this is the id from the application registered with FusionAuth
    client_id: Secret<String>,
    // this is the secret generated by FusionAuth
    client_secret: Secret<String>,
}

#[get("/")]
async fn hello(
    app_data: web::Data<YogaAppData>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    // OAuth flow step 2.
    // The client redirects your browser to the authorization server.
    // It includes with the request, the client id, redirect uri, the response type, and one or more scopes it needs.

    let auth_url = "aquiles.local";
    let client_id = app_data.client_id.expose_secret();
    let state = session.generate_and_save_state().unwrap();
    let url_encoded_code_challenge = session.generate_and_save_code_challenge().unwrap();
    let redirect_ip = "matts-imac.local";
    let redirect_port = "3000";
    let login_uri = format!("http://{}", auth_url);
    let login_uri = format!(
        "{}:9011/oauth2/authorize?client_id={}",
        login_uri, client_id
    );
    let login_uri = format!(
        "{}&response_type=code&redirect_uri=http%3A%2F%2F{}%3A{}",
        login_uri, redirect_ip, redirect_port
    );
    let login_uri = format!(
        "{}%2Foauth-redirect&code_challenge={}&code_challenge_method=S256",
        login_uri, url_encoded_code_challenge
    );

    let nice_uri = format!("http://{}:9011/oauth2/authorize?", auth_url)
     + &format!("client_id={}&", client_id)
     + &format!("response_type=code&")
     + &format!("redirect_uri=")
     + &urlencoding::encode(&format!("http://{}:{}/", redirect_ip, redirect_port))
     + &format!("oauth-redirect&")
     + &format!("code_challenge={}&", url_encoded_code_challenge)
     + &format!("code_challenge_method=S256");

    use pretty_assertions::assert_eq;
    assert_eq!(nice_uri, login_uri);

    // a(href=fusionAuthURL+'/oauth2/logout/?client_id='+clientId) Logout

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>
<body>
Hello world! <a href={}>Login</a>
</body>
</html>"#,
            login_uri
        )))
}

// OAuth flow step 5.
// The authorization server redirects back to the client using the redirect uri.
// Along with a temporary authorization code.

#[derive(serde::Deserialize)]
pub struct LoginRedirect {
    code: String,
    #[serde(alias = "userState")]
    user_state: String,
}

async fn oauth_login_redirect(
    login: web::Query<LoginRedirect>,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    // redirect from the authorization server
    // code - authorization code the OAuth server created after the user logged in
    // it needs to be exchanged for tokens
    // state - this is the same value of the state parameter we passed to the OAuth server
    // this is echoed back to this application so that we can verify that the code
    // came from the correct location

    if let Some(state_cookie) = session.get::<String>("state_value")? {
        println!("session state_value: {}", state_cookie);
        println!("login code:       {}", login.code);
        println!("login user_state: {}", login.user_state);
        if login.code != state_cookie {
            let code_str = format!("got:      {}", login.code);
            let expc_str = format!("expected: {}", state_cookie);

            tracing::info!("State doesn't match.\n{}\n{}", code_str, expc_str);
            //res.redirect(302, '/');
            //return
        } else {
            tracing::info!("state matches yeah!");
        }
    } else {
        tracing::info!("no `state_value` in the session");
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Logged In</title>
</head>
<body>
<p>You have been logged in.</p>
<p>user_state {}</p>
<p>code {}</p>
</body>
</html>"#,
            login.user_state, login.code
        )))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    dotenv::dotenv().ok();
    let client_id = dotenv::var("CLIENT_ID").unwrap().into();
    let client_secret = dotenv::var("CLIENT_SECRET").unwrap().into();

    let yoga_data = web::Data::new(YogaAppData {
        client_id,
        client_secret,
    });

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(hello)
            .route("/oauth-redirect", web::get().to(oauth_login_redirect))
            .app_data(yoga_data.clone())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                    .cookie_secure(false)
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(cookie::time::Duration::hours(2)),
                    )
                    .build(),
            )
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await
}

// http://127.0.0.1:8080/
