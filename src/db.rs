use crate::{error::Error::*, handler::BookRequest, Book, Result};

use futures::StreamExt;

use chrono::Utc;

use mongodb::bson::{doc, document::Document, oid::ObjectId, Bson};
use mongodb::{options::ClientOptions, Client, Collection};

const DB_NAME: &str = "booky";
const COLL: &str = "books";

const ID: &str = "_id";
const NAME: &str = "name";
const AUTHOR: &str = "author";
const NUM_PAGES: &str = "num_pages";
const ADDED_AT: &str = "added_at";
const TAGS: &str = "tags";
const APP_NAME: &str = "booky_app";

#[derive(Debug, Clone)]
pub struct DB {
    pub client: Client,
}

impl DB {
    pub async fn init() -> Result<Self> {
        let mut client_options = ClientOptions::parse("mongodb://127.0.0.1:27017").await?;
        client_options.app_name = Some(APP_NAME.to_string());

        Ok(Self {
            client: Client::with_options(client_options)?,
        })
    }

    pub async fn fetch_books(&self) -> Result<Vec<Book>> {
        let mut cursor = self
            .get_collection()
            .find(None, None)
            .await
            .map_err(MongoQueryError)?;

        let mut result: Vec<Book> = Vec::new();
        while let Some(doc) = cursor.next().await {
            result.push(self.doc_to_book(&doc?)?);
        }
        Ok(result)
    }

    pub async fn create_book(&self, entry: &BookRequest) -> Result<()> {
        let doc = doc! {
            NAME: entry.name.clone(),
            AUTHOR: entry.author.clone(),
            NUM_PAGES: entry.num_pages as i32,
            ADDED_AT: Utc::now(),
            TAGS: entry.tags.clone(),
        };
        self.get_collection()
            .insert_one(doc, None)
            .await
            .map_err(MongoQueryError)?;
        Ok(())
    }

    pub async fn edit_book(&self, id: &str, entry: &BookRequest) -> Result<()> {
        // https://jira.mongodb.org/browse/RUST-1209#:~:text=The%20short%20answer%3A%20the%20behavior%20of%20the%20Rust,using%20%24set%20with%20update_one%20%28%29%20as%20you%20describe.
        let oid = ObjectId::parse_str(id).map_err(|_| InvalidIDError(id.to_owned()))?;
        let query = doc! { "_id": oid };
        let doc = doc! {
            NAME: entry.name.clone(),
            AUTHOR: entry.author.clone(),
            NUM_PAGES: entry.num_pages as i32,
            ADDED_AT:  Utc::now(),
            TAGS: entry.tags.clone(),
        };

        self.get_collection()
            .replace_one(query, doc, None)
            .await
            .map_err(MongoQueryError)?;

        Ok(())
    }

    pub async fn delete_book(&self, id: &str) -> Result<()> {
        let oid = ObjectId::parse_str(id).map_err(|_| InvalidIDError(id.to_owned()))?;
        let filter = doc! { "_id": oid };
        self.get_collection()
            .delete_one(filter, None)
            .await
            .map_err(MongoQueryError)?;
        Ok(())
    }

    fn get_collection(&self) -> Collection<Document> {
        self.client.database(DB_NAME).collection(COLL)
    }

    fn doc_to_book(&self, doc: &Document) -> Result<Book> {
        let id = doc.get_object_id(ID)?;
        let name = doc.get_str(NAME)?;
        let author = doc.get_str(AUTHOR)?;
        let num_pages = doc.get_i32(NUM_PAGES)?;
        let added_at = doc.get_datetime(ADDED_AT)?;
        let tags = doc.get_array(TAGS)?;

        let book = Book {
            id: id.to_hex(),
            name: name.to_owned(),
            author: author.to_owned(),
            num_pages: num_pages as usize,
            added_at: (*added_at).into(),
            tags: tags
                .iter()
                .filter_map(|entry| match entry {
                    Bson::String(v) => Some(v.to_owned()),
                    _ => None,
                })
                .collect(),
        };
        Ok(book)
    }
}
