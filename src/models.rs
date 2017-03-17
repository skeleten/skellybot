use ::schema::*;

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub discord_id: i64,
    pub last_seen: Option<::std::time::SystemTime>
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser {
    pub discord_id: i64,
    pub last_seen: Option<::std::time::SystemTime>,
}
