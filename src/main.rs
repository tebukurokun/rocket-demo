use anyhow::Result;
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Object, Schema,
};

use async_graphql_rocket::{GraphQLQuery, GraphQLRequest, GraphQLResponse};
use rocket::{response::content, State};
use sqlx::{postgres::PgPoolOptions, PgPool};

struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn answer(&self, ctx: &async_graphql::Context<'_>) -> Result<i32> {
        let pool = ctx
            .data::<PgPool>()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let (answer,): (i32,) = sqlx::query_as("select 20").fetch_one(pool).await?;
        Ok(answer)
    }
}

type StarWarsSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

#[rocket::get("/")]
fn graphql_playground() -> content::Html<String> {
    content::Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

#[rocket::get("/graphql?<query..>")]
async fn graphql_query(schema: &State<StarWarsSchema>, query: GraphQLQuery) -> GraphQLResponse {
    query.execute(schema).await
}

#[rocket::post("/graphql", data = "<request>", format = "application/json")]
async fn graphql_request(
    schema: &State<StarWarsSchema>,
    request: GraphQLRequest,
) -> GraphQLResponse {
    request.execute(schema).await
}

#[rocket::launch]
async fn rocket() -> _ {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://user:password@localhost:54321/person_db")
        .await
        .unwrap();
    let query_root = QueryRoot;
    let schema = Schema::build(query_root, EmptyMutation, EmptySubscription)
        .data(pool)
        .finish();

    rocket::build().manage(schema).mount(
        "/",
        rocket::routes![graphql_query, graphql_request, graphql_playground],
    )
}
