#![allow(non_snake_case)]

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct APIQuery {
  pub bg_file: String,
}