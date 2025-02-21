#![recursion_limit = "256"]

use jplaw_data_types::article::ArticleIndex;
use jplaw_data_types::law::LawId;
use jplaw_data_types::precedent::PrecedentInfo;
use meilisearch_sdk::client::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct Precedent {
    pub id: String,
    pub info: PrecedentInfo,
    pub text: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum LegalDocument {
    Law(Law),
    Precedent(Precedent),
}

impl LegalDocument {
    pub fn get_id(&self) -> String {
        match self {
            LegalDocument::Law(l) => l.id.clone(),
            LegalDocument::Precedent(p) => p.id.clone(),
        }
    }
    pub fn get_text(&self) -> String {
        match self {
            LegalDocument::Law(l) => l.text.clone(),
            LegalDocument::Precedent(p) => p.text.clone(),
        }
    }
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
        let client = Client::new(meilisearch_url, Some(masterkey))
            .map_err(|_| LawscapeCoreError::MeilisearchClientError)?;
        Ok(Self {
            meilisearch_client: client,
        })
    }

    // 検索用レジストリにデータを追加する
    pub async fn add_data(&self, data: &[LegalDocument]) -> Result<(), LawscapeCoreError> {
        let index = self.meilisearch_client.index(REGISTORY_INDEX_NAME);
        index
            .add_documents(data, Some(REGISTORY_ID_NAME))
            .await
            .map_err(|_| LawscapeCoreError::MeilisearchIndexError)?;
        Ok(())
    }

    /// 検索用レジストリから値を取得する
    pub async fn search(&self, word: &str) -> Result<Vec<LegalDocument>, LawscapeCoreError> {
        let index = self.meilisearch_client.index(REGISTORY_INDEX_NAME);
        let mut result = index
            .search()
            .with_query(word)
            .with_limit(100000)
            .with_locales(&["jpn"])
            .with_ranking_score_threshold(0.5)
            .execute::<LegalDocument>()
            .await
            .map_err(|_| LawscapeCoreError::MeilisearchSearchError)?
            .hits;
        result.sort_by(|t1, t2| t2.ranking_score.partial_cmp(&t1.ranking_score).unwrap());
        let document_list = result
            .iter()
            .take(1000)
            .map(|search_result| search_result.clone().result)
            .collect();
        Ok(document_list)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalDocumentDependencies {
    /// 同じ法令に含まれる法令文書など。判例の場合は通常一つ。
    pub contents: Vec<LegalDocument>,
    /// 参照している法令文書
    pub parents: Vec<String>,
    /// 参照されている法令文書
    pub children: Vec<String>,
}

pub fn analyze_search_result_dependencies(
    legal_documents: &[LegalDocument],
) -> HashMap<String, LegalDocumentDependencies> {
    let mut id_list = legal_documents
        .iter()
        .map(|d| d.get_id())
        .collect::<Vec<String>>();
    id_list.sort();
    id_list.dedup();
    let mut contents_list = Vec::new();
    for id in id_list.iter() {
        let documents = legal_documents
            .iter()
            .filter(|d| &d.get_id() == id)
            .cloned()
            .collect::<Vec<_>>();
        let document = documents.first();
        let name = match document {
            Some(LegalDocument::Law(l)) => Some(l.name.clone()),
            _ => None,
        };
        contents_list.push((id, name, documents.clone()));
    }
    let mut paretns_list: Vec<Vec<String>> = vec![Vec::new(); id_list.len()];
    let mut children_list: Vec<Vec<String>> = vec![Vec::new(); id_list.len()];
    for (i, (id, name, _)) in contents_list.iter().enumerate() {
        if let Some(name) = name {
            for (j, (id2, name2, documents2)) in contents_list.iter().enumerate() {
                if id == id2 {
                    continue;
                };
                // 法令名称が含まれるかどうかの判定
                let mut is_contains = false;
                if let Some(name2) = name2 {
                    is_contains = name2.contains(name);
                }
                for document2 in documents2.iter() {
                    if is_contains {
                        break;
                    }
                    is_contains = document2.get_text().contains(name);
                }
                if is_contains {
                    // idが親でid2が子にあたる
                    paretns_list[j].push(id.to_string());
                    children_list[i].push(id2.to_string());
                }
            }
        }
    }
    let mut document_dependencies: HashMap<String, LegalDocumentDependencies> = HashMap::new();
    for (i, (id, _, contents)) in contents_list.iter().enumerate() {
        let d = LegalDocumentDependencies {
            contents: contents.clone(),
            parents: paretns_list[i].clone(),
            children: children_list[i].clone(),
        };
        document_dependencies.insert(id.to_string(), d);
    }
    document_dependencies
}
