//! Shared DynamoDB article schema.
//!
//! Both the web app (`typing`) and the `article-import` tool read and write
//! these types, so the shape stays identical on both sides of the pipeline.

pub use article_model::*;
