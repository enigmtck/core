use enigmatick::retriever::collection_fetcher;
use futures_lite::StreamExt;
use jdt_activity_pub::ApCollection;
use jdt_activity_pub::Collectible;

#[tokio::main]
pub async fn main() {
    let client = reqwest::Client::new();
    let collection = client
        //.get("https://mastodon.social/users/Gargron/outbox")
        .get("https://mastodon.social/users/belldotbz/statuses/114754909171422185/replies")
        //.get("https://enigmatick.social/user/jdt/outbox")
        .header("Content-Type", "application/activity+json")
        .send()
        .await
        .unwrap()
        .json::<ApCollection>()
        .await
        .unwrap();

    println!("Collection: {collection:?}");
    println!("Items count: {:?}", collection.items().map(|i| i.len()));
    println!("First: {:?}", collection.first);
    println!("Next: {:?}", collection.next);

    let mut stream = collection.stream_all(collection_fetcher());
    while let Some(item_result) = stream.next().await {
        match item_result {
            Ok(item) => println!("Got item: {item:?}"),
            Err(e) => eprintln!("Error fetching: {e}"),
        }
    }
}
