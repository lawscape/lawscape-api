#![recursion_limit = "256"]

use serde::{Deserialize, Serialize};
use jplaw_data_types::precedent::PrecedentInfo;
use jplaw_data_types::law::LawId;
use jplaw_data_types::article::ArticleIndex;
use meilisearch_sdk::client::Client;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum LawscapeCoreError {
  #[error("meilisearch client error")]
  MeilisearchClientError,
  #[error("meilisearch index error")]
  MeilisearchIndexError,
  #[error("meilisearch client error")]
  MeilisearchSearchError,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct Law {
  pub id: String,
  pub law_id: LawId,
  pub name: String,
  pub index: ArticleIndex,
  pub text: String
}


#[derive(Debug, Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct Precedent {
  pub id: String,
  pub info: PrecedentInfo,
  pub text: String
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum LegalDocument {
  Law(Law),
  Precedent(Precedent),
}

#[derive(Debug, Clone)]
pub struct LegalDocumentsRegistory {
  meilisearch_client: Client,
}

const REGISTORY_INDEX_NAME: &str = "legal_documents";
const REGISTORY_ID_NAME: &str = "id";

impl LegalDocumentsRegistory {

  /// 検索レジストリへのアクセスを生成
  pub fn new(meilisearch_url: &str, masterkey: &str) -> Result<Self, LawscapeCoreError> {
    let client = Client::new(meilisearch_url, Some(masterkey)).map_err(|_| LawscapeCoreError::MeilisearchClientError)?;
    Ok(Self { meilisearch_client: client })
  }

  // 検索用レジストリにデータを追加する
  pub async fn add_data(&self, data: &[LegalDocument]) -> Result<(), LawscapeCoreError> {
    let index = self.meilisearch_client.index(REGISTORY_INDEX_NAME);
    index.add_documents(data, Some(REGISTORY_ID_NAME)).await.map_err(|_| LawscapeCoreError::MeilisearchIndexError)?;
    Ok(())
  }

  /// 検索用レジストリから値を取得する
  pub async fn search(&self, word: &str) -> Result<Vec<LegalDocument>, LawscapeCoreError> {
    let index = self.meilisearch_client.index(REGISTORY_INDEX_NAME);
    let result = index.search().with_query(word).execute::<LegalDocument>().await.map_err(|_| LawscapeCoreError::MeilisearchSearchError)?.hits;
    let document_list = result.iter().map(|search_result| search_result.clone().result).collect();
    Ok(document_list)
  }
}


