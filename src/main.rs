use anyhow::Result;
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, InputObject, Object, Schema, SimpleObject,
};

use async_graphql_rocket::{GraphQLQuery, GraphQLRequest, GraphQLResponse};
use chrono::NaiveDateTime;
use rocket::{response::content, State};
use sqlx::prelude::*;
use sqlx::{postgres::PgPoolOptions, PgPool};

struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn answer(&self, ctx: &async_graphql::Context<'_>) -> Result<i32, async_graphql::Error> {
        let pool = ctx.data::<PgPool>()?;
        let (answer,): (i32,) = sqlx::query_as("select 20").fetch_one(pool).await?;
        Ok(answer)
    }
}

#[derive(SimpleObject)]
struct Person {
    id: i32,
    name: String,
    age: i32,
}

#[derive(InputObject)]
struct CreatePersonInput {
    name: String,
    age: i32,
}

#[derive(Debug, FromRow)]
struct PersonRecord {
    id: i32,
    name: String,
    age: i32,
    created_at: NaiveDateTime,
}

struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_person(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: CreatePersonInput,
    ) -> Result<Person, async_graphql::Error> {
        let pool = ctx.data::<PgPool>()?;
        let mut tx = pool.begin().await?;
        let sql = "
            insert into person (
                name, age, created_at
            ) 
            values 
            (
                $1, $2, current_timestamp
            ) 
            returning id, name, age, created_at
            ;
        ";

        let person_record: PersonRecord = sqlx::query_as(sql)
            .bind(input.name)
            .bind(input.age)
            .fetch_one(&mut tx)
            .await?;

        let gql_person = Person {
            id: person_record.id,
            name: person_record.name,
            age: person_record.age,
        };

        tx.commit().await?;

        Ok(gql_person)
    }
}

type PersonSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[rocket::get("/")]
fn graphql_playground() -> content::Html<String> {
    content::Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

#[rocket::get("/graphql?<query..>")]
async fn graphql_query(schema: &State<PersonSchema>, query: GraphQLQuery) -> GraphQLResponse {
    query.execute(schema).await
}

#[rocket::post("/graphql", data = "<request>", format = "application/json")]
async fn graphql_request(schema: &State<PersonSchema>, request: GraphQLRequest) -> GraphQLResponse {
    request.execute(schema).await
}

#[rocket::launch]
async fn rocket() -> _ {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://user:password@localhost:54321/person_db")
        .await
        .unwrap();
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(pool)
        .finish();

    rocket::build().manage(schema).mount(
        "/",
        rocket::routes![graphql_query, graphql_request, graphql_playground],
    )
}
