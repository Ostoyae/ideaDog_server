use arangors::AqlQuery;
use r2d2::PooledConnection;
use r2d2_arangodb::ArangodbConnectionManager;
use serde::Deserialize;
use serde::Serialize;

type Connection = PooledConnection<ArangodbConnectionManager>;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Owner {
    #[serde(alias = "_key")]
    pub id: String,
    #[serde(alias = "name")]
    pub username: String,
}

impl Owner {
    /// This method will fetch Owner (User) from the Database
    ///
    /// # Errors
    /// Error occurse if failed to connect to database or Owner (User) doesn't exist
    ///
    pub fn get_owner(id: String, conn: &Connection) -> Option<Owner> {
        let ident = if id.contains('/') {
            id
        } else {
            format!("users/{}", id)
        };

        let aql = AqlQuery::new("RETURN DOCUMENT(@ident)")
            .bind_var("ident", ident)
            .batch_size(1);

        let owner = match conn.aql_query(aql) {
            Ok(mut r) => Some(r.pop().unwrap()),
            Err(e) => {
                println!("Error: {}", e);
                None
            }
        };

        owner
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Idea {
    // _id field from arangodb
    #[serde(alias = "_id")]
    pub id: String,
    // _key field from arangodb
    #[serde(alias = "_key")]
    pub key: String,
    // title of the idea
    pub text: String,
    // description of idea
    // Owner's username
    //    #[serde(skip)]
    pub owner: Owner,
    // This field is for the votes.
    #[serde(default)]
    pub upvotes: i32,
    #[serde(default)]
    pub downvotes: i32,

    pub tags: Vec<String>,

    pub date: i64,
}

#[derive(Debug, Clone)]
pub struct NewIdea {
    // title of the idea
    pub text: String,
    //    #[serde(default="temp_user")]
    // Owner's username
    pub owner_id: String,

    pub tags: Vec<String>,
}

#[derive(Debug)]
pub enum Sort {
    ALL,
    BRIGHT,
}

impl Default for Sort {
    fn default() -> Self {
        Sort::ALL
    }
}
#[derive(Debug, Default)]
pub struct QueryIdea {
    pub sort: Sort,
    //id
    pub id: Option<String>,
    // Owner's username
    pub owner: Option<String>,
    // Owner's string id
    pub owner_id: Option<String>,
    // accept tags for query string
    pub tags: Option<Vec<String>>,

    pub pagination: Option<Pagination>,
    // query search
    pub query: Option<String>,
}

#[derive(Debug)]
pub struct Pagination {
    pub count: u32,
    pub offset: u32,
}

#[derive(Clone)]
pub struct CastVote {
    pub idea_id: String,
    pub u_token: String,
    pub vote: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VoteStatus {
    pub idea_id: String,
    pub prev: Option<String>,
    pub new: Option<String>,
}

impl VoteStatus {
    pub fn has_changed(&self) -> bool {
        if self.prev != self.new {
            true
        } else {
            false
        }
    }
}
