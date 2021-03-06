use actix::{Handler, MailboxError};
use arangors;
use arangors::AqlQuery;

use r2d2::Error;

use serde_json;

use crate::{DbExecutor, NewUser, QUser, User};

impl Handler<QUser> for DbExecutor {
    type Result = Result<User, MailboxError>;

    //noinspection RsExternalLinter
    fn handle(&mut self, msg: QUser, _ctx: &mut Self::Context) -> Self::Result {
        let conn = self.0.get().unwrap();

        let mut aql;

        match msg {
            QUser::TOKEN(tok) => {
                aql = AqlQuery::new(
"
let u = FIRST (FOR u in 1..1 OUTBOUND DOCUMENT('bearer_tokens', @ele) bearer_to_user RETURN u)
let ideas = (FOR i in 1..1 INBOUND u._id idea_owner
return {[i._key]: 'true'}
)
let c = FIRST (FOR i in 1..1 INBOUND u._id idea_owner
Collect AGGREGATE upvotes = SUM(i.upvotes), downvotes = SUM(i.downvotes)
return {upvotes, downvotes}
)
let votes = (FOR v, e in 1..1 INBOUND u._id idea_voter RETURN {[v._key]: e.vote })
return Merge(u, {ideas: MERGE(ideas) ,votes: MERGE(votes), upvotes: TO_NUMBER(c.upvotes), downvotes: TO_NUMBER(c.downvotes)})
",
                )
                .bind_var("ele", tok.clone())
                .batch_size(1);
            }

            QUser::ID(id) => {
                aql = AqlQuery::new(
"let u = DOCUMENT('users', @ele)
let ideas = (FOR i in 1..1 INBOUND u._id idea_owner
return {[i._key]: 'true'}
)
let c = FIRST (FOR i in 1..1 INBOUND u._id idea_owner
Collect AGGREGATE upvotes = SUM(i.upvotes), downvotes = SUM(i.downvotes)
return {upvotes, downvotes}
)

return Merge(u, {ideas: MERGE(ideas) , upvotes: TO_NUMBER(c.upvotes), downvotes: TO_NUMBER(c.downvotes)})
")
                    .bind_var("ele", id.clone())
                    .batch_size(1);
            }
        }

        let response: Result<User, MailboxError> = match conn.aql_query(aql) {
            Ok(mut r) => {
                if !r.is_empty() {
                    Ok(r.pop().unwrap())
                } else {
                    Err(MailboxError::Closed)
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                Err(MailboxError::Closed)
            }
        };

        response
    }
}

impl Handler<NewUser> for DbExecutor {
    type Result = Result<Vec<User>, Error>;

    fn handle(&mut self, msg: NewUser, _ctx: &mut Self::Context) -> Self::Result {
        let conn = self.0.get().unwrap();
        let data = serde_json::to_value(msg.clone()).unwrap();
        let aql = AqlQuery::new("INSERT @data INTO users LET n = NEW RETURN n")
            .bind_var("data", data)
            .batch_size(1);
        let response = match conn.aql_query(aql) {
            Ok(r) => r,
            Err(e) => {
                println!("Error: {}", e);
                vec![]
            }
        };

        Ok(response)
    }
}
