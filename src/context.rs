use ::diesel::pg::PgConnection;
use ::diesel::*;
use ::discord;
use ::error::*;

pub struct Context {
    pub client: discord::Discord,
    pub servers: Vec<discord::model::ServerId>,
    pub postgres_url_overwrite: Option<String>
}

impl Context {
    pub fn new(client: discord::Discord) -> Self {
        Context {
            client: client,
            servers: Vec::new(),
            postgres_url_overwrite: None,
        }
    }

    pub fn get_postgres_url(&self) -> Result<String> {
        if let Some(ref url) = self.postgres_url_overwrite {
            Ok(url.to_owned())
        } else {
            let db_url = ::std::env::var("DATABASE_URL")
                .chain_err(|| "DATABASE_URL must be set!")?;
            Ok(db_url)
        }
    }

    pub fn establish_connection(&self) -> Result<PgConnection> {
        use ::diesel::Connection;
        let db_url = self.get_postgres_url()?;
        PgConnection::establish(&db_url)
            .chain_err(|| format!("Error connecting to {}", db_url))
    }

    pub fn user_seen(&self, uid: &::discord::model::UserId) -> Result<()> {
        use ::discord::model::UserId;
        use ::schema::users::dsl::*;
        println!("updating user id {}", uid);
        let &UserId(uid) = uid;
        let uid = uid as i64;
        let conn = self.establish_connection()?;
        let results = users.filter(discord_id.eq(uid))
            .limit(1)
            .load::<::models::User>(&conn)
            .chain_err(|| "Failed to load users")?;

        if results.len() >= 1 {
            let dbuid = results[0].id;
            let user = ::diesel::update(users.find(id))
                .set(last_seen.eq(Some(::std::time::SystemTime::now())))
                .get_result::<::models::User>(&conn)
                .chain_err(|| "Failed to update user")?;
        } else {
            Self::create_user(&conn, uid, Some(::std::time::SystemTime::now()))?;
        };
        Ok(())
    }

    pub fn create_user(conn: &PgConnection,
                       discord_id: i64,
                       last_seen: Option<::std::time::SystemTime>)
                       -> Result<::models::User> {
        use schema::users;

        let new_user = ::models::NewUser {
            discord_id: discord_id,
            last_seen: last_seen,
        };

        ::diesel::insert(&new_user)
            .into(users::table)
            .get_result(conn)
            .chain_err(|| "Error saving new user")
    }
}
