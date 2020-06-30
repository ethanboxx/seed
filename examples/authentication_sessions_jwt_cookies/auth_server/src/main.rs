use {
    chrono::{offset::Utc, Duration as ChronoDuration},
    cookie::Cookie,
    jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation},
    serde::{Deserialize, Serialize},
    tide::{http::StatusCode, Body, Redirect, Request, Response},
    time::Duration as TimeDuration,
};

// This secret is used to encode and decode tokens.
// It MUST be secret. I recommend storing it as an environment variable rather than a
// constant to avoid pushing secrets to repositories.
const SECRET: &'static str = "SECRET";

const CLIENT: &'static str = "http://localhost:8000";

#[async_std::main]
async fn main() -> tide::Result<()> {
    let mut app = tide::new();

    // femme will print logs for us.
    femme::with_level(log::LevelFilter::Debug);

    app.at("/sign-in").get(sign_in);
    app.at("/sign-out").get(sign_out);
    app.at("/signed-in").get(signed_in);

    app.listen("localhost:8081").await?;

    Ok(())
}

// Sign in a user.
async fn sign_in(_: Request<()>) -> tide::Result<Response> {
    // Lets set the response to redirect to our PWA server once the cookie is provided.
    let mut response: Response = Redirect::new(CLIENT).into();
    // We will give the user the username "nori". We will not bother with a password for
    // nori yet as that would be out of the scope of this example.
    response.insert_cookie(
        Cookie::build("login", Claims::new("nori".to_owned()).get_token()?)
            // Let's make sure that this cookie is only sent over a secure connection.
            .secure(true)
            .http_only(true)
            // The token will only be valid for a day so let's set the `max-age` of the
            // cookie to reflect this.
            .max_age(TimeDuration::days(Claims::max_age_days()))
            .finish(),
    );
    Ok(response)
}

// Simply "signs out" a user by removing the token cookie.
async fn sign_out(_: Request<()>) -> tide::Result<Response> {
    let mut res: Response = Redirect::new(CLIENT).into();
    res.remove_cookie(Cookie::named("login"));
    Ok(res)
}

// Checks if a user is signed in.
async fn signed_in(req: Request<()>) -> tide::Result<Response> {
    let user = req.cookie("login").and_then(|cookie| {
        Claims::decode_token(cookie.value())
            .map(|token| token.claims.sub)
            .ok()
    });

    let mut res = Response::new(StatusCode::Ok);
    res.set_body(Body::from_json(&match user {
        Some(_user) => true,
        None => false,
    })?);
    Ok(res)
}

// `Claims` is the data we are going to encode in our tokens.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    // Sub is where we are going to store the username of the user.
    pub sub: String,
    company: String,
    expires: i64,
}

impl Claims {
    // Generates a Claim for a user.
    fn new(user: String) -> Self {
        Self {
            sub: user,
            company: "example.com".to_owned(),
            // Let's set the token to expire in one day.
            expires: (Utc::now() + ChronoDuration::days(Self::max_age_days())).timestamp()
            ,
        }
    }
    // Generates a token from a claim.
    fn get_token(&self) -> jsonwebtoken::errors::Result<String> {
        encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
    }
    // Decodes a token to produce the underlying claim.
    fn decode_token(token: &str) -> Result<TokenData<Self>, jsonwebtoken::errors::Error> {
        use jsonwebtoken::errors::ErrorKind;

        let token = decode::<Claims>(
            token,
            &DecodingKey::from_secret(SECRET.as_bytes()),
            &Validation::default(),
        );

        if let Ok(ref token) = token {
            // If the token is expired then let's return an error.
            if token.claims.expires < Utc::now().timestamp() {
                return Err(ErrorKind::InvalidToken.into());
            }
        }
        token
    }
    /// Can't simply return a `time::Duration` due to time crate version miss-match with `chrono` and `cookie`
    const fn max_age_days() -> i64 {
        1
    }
}