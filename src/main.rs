use std::{env, error::Error, sync::Arc};

use bson::{doc, to_vec};

use async_graphql::{
    Context, EmptySubscription, InputObject, Object, Schema, http::GraphiQLSource,
};
use async_graphql_poem::*;
use poem::{listener::TcpListener, web::Html, *};

use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties, options::*, types::FieldTable,
};

use serde::{Deserialize, Serialize};

#[derive(InputObject)]
struct NewUser {
    username: String,
    email: String,
}

#[derive(Deserialize, Serialize)]
struct NewUserMessage {
    username: String,
    email: String,
}

struct Query;
struct Mutation;

#[Object]
impl Query {
    async fn howdy(&self) -> &'static str {
        "partner"
    }
}

#[Object]
impl Mutation {
    async fn create_user(&self, context: &Context<'_>, input: NewUser) -> bool {
        let channel = context.data_unchecked::<Arc<Channel>>();

        let message = NewUserMessage {
            username: input.username.clone(),
            email: input.email.clone(),
        };

        let serialized = bson::to_document(&message).expect("Failed to serialize");
        let payload = to_vec(&serialized).expect("Failed to serialize");

        channel
            .basic_publish(
                "user_exchange",
                "user.new",
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await
            .expect("Failed to publish")
            .await
            .expect("Failed to confirm");

        true
    }
}

#[handler]
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().finish())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let address =
        env::var("RABBITMQ_URL").unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".into());

    let connection = Connection::connect(&address, ConnectionProperties::default()).await?;
    let channel = connection.create_channel().await?;

    channel
        .exchange_declare(
            "user_exchange",
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    // create the schema
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(Arc::new(channel.clone()))
        .finish();

    // start the http server
    let app = Route::new().at("/", get(graphiql).post(GraphQL::new(schema)));
    println!("GraphiQL: http://localhost:8000");
    Server::new(TcpListener::bind("0.0.0.0:8000"))
        .run(app)
        .await?;

    Ok(())
}
