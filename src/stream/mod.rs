use std::pin::Pin;
use bytes::Bytes;
use tokio_stream::Stream;
use crate::error::GitInnerError;

pub struct DataStream {
    pub input: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>> + Send + 'static>>,
    pub output: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>> + Send + 'static>>,
    pub error: Pin<Box<dyn Stream<Item = Result<Bytes, GitInnerError>> + Send + 'static>>,
    pub done: bool,
}

impl DataStream {
    pub fn new() -> Self {
        Self {
            input: Box::pin(tokio_stream::empty()),
            output: Box::pin(tokio_stream::empty()),
            error: Box::pin(tokio_stream::empty()),
            done: false,
        }
    }
}