use anyhow::{Result, anyhow};
use clap::Parser;
use jplaw_data_types::{
    article,
    law::{Date, LawPatchInfo},
    listup::{LawInfo, PrecedentInfo},
    precedent::PrecedentData,
};
use lawscape_core::{Law, LegalDocument, LegalDocumentsRegistory, Precedent};
use regex::Regex;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio_stream::StreamExt;
use tracing::info;

fn parse_date(str: &str) -> Result<Date> {
    let re1 = Regex::new("(?<y>[0-9]{4})/(?<m>[0-9]{2})/(?<d>[0-9]{2})").unwrap();
    let re2 = Regex::new("(?<y>[0-9]{4})-(?<m>[0-9]{2})-(?<d>[0-9]{2})").unwrap();
    let re3 = Regex::new("(?<y>[0-9]{4})(?<m>[0-9]{2})(?<d>[0-9]{2})").unwrap();

    if let Some(caps) = re1.captures(str) {
        let y = &caps["y"];
        let y = y.parse::<usize>()?;
        let m = &caps["m"];
        let m = m.parse::<usize>()?;
        let d = &caps["d"];
        let d = d.parse::<usize>()?;
        if 12 < m || 31 < d {
            Err(anyhow!("日付が範囲外です"))
        } else {
            Ok(Date::gen_from_ad(y, m, d))
        }
    } else if let Some(caps) = re2.captures(str) {
        let y = &caps["y"];
        let y = y.parse::<usize>()?;
        let m = &caps["m"];
        let m = m.parse::<usize>()?;
        let d = &caps["d"];
        let d = d.parse::<usize>()?;
        if 12 < m || 31 < d {
            return Err(anyhow!("日付が範囲外です"));
        } else {
            Ok(Date::gen_from_ad(y, m, d))
        }
    } else if let Some(caps) = re3.captures(str) {
        let y = &caps["y"];
        let y = y.parse::<usize>()?;
        let m = &caps["m"];
        let m = m.parse::<usize>()?;
        let d = &caps["d"];
        let d = d.parse::<usize>()?;
        if 12 < m || 31 < d {
            return Err(anyhow!("日付が範囲外です"));
        } else {
            Ok(Date::gen_from_ad(y, m, d))
        }
    } else {
        Err(anyhow!(
            "対応していない日付のフォーマットです。対応フォーマット：yyyy/MM/dd, yyyy-MM-dd, yyyyMMdd"
        ))
    }
}

/// ログを出力するための設定など
async fn init_logger() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

#[derive(Debug, Parser)]
struct AppArg {
    /// meilisearchのURL
    #[arg(long, env = "MEILISEARCH_URL")]
    pub meilisearch_url: String,
    /// meilisearchのmaster key
    #[arg(long, env = "MEILISEARCH_MASTER_KEY", hide_env_values = true)]
    pub meilisearch_master_key: String,
    /// 法令データのXMLが入ったフォルダ
    #[arg(long)]
    pub law_folder: String,
    /// 法令データのインデックス
    #[arg(long)]
    pub law_index: String,
    /// 法令データのテキストファイルが入ったフォルダ
    #[arg(long)]
    pub precedent_folder: String,
    /// 法令データのインデックス
    #[arg(long)]
    pub precedent_index: String,
    /// 法律を登録する際の基準とする日付
    #[arg(long)]
    pub date: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let app_args = AppArg::parse();

    init_logger().await?;

    let law_date = parse_date(&app_args.date)?;

    let legal_document_registory =
        LegalDocumentsRegistory::new(&app_args.meilisearch_url, &app_args.meilisearch_master_key)?;

    info!("[START] parsing law data");
    // 検索エンジン用の法令データを生成する
    let mut law_index_f = File::open(app_args.law_index).await?;
    let mut law_index_buf = Vec::new();
    law_index_f.read_to_end(&mut law_index_buf).await?;
    let law_index_list = serde_json::from_slice::<Vec<LawInfo>>(&law_index_buf)?;
    let mut law_index_stream = tokio_stream::iter(law_index_list);
    while let Some(law_index) = law_index_stream.next().await {
        let id = &law_index.id;
        let id_str = format!("{id}");
        let name = &law_index.name;

        // 与えられた日付時点で施行されている物を探す
        let mut patch_file: Option<LawPatchInfo> = None;
        for law_patch in law_index.patch.iter() {
            if law_patch.patch_date < law_date {
                if let Some(p) = &patch_file {
                    if law_patch.patch_date > p.patch_date {
                        patch_file = Some(law_patch.clone())
                    }
                } else {
                    patch_file = Some(law_patch.clone())
                }
            }
        }
        if let Some(law_file_name) = patch_file.map(|p| p.to_file_path()) {
            info!("[START] parsing law: {id_str}");
            let law_file_path = Path::new(&app_args.law_folder);
            let law_file_path = law_file_path.join(&law_file_name);
            let law_file_path = law_file_path.join(format!("{law_file_name}.xml"));
            let mut law_xml_f = File::open(law_file_path).await?;
            let mut law_xml_buf = Vec::new();
            law_xml_f.read_to_end(&mut law_xml_buf).await?;
            let law_data = japanese_law_xml_schema::parse_xml(&law_xml_buf)?;
            let article_list =
                article::article_list_from_lawbody(&id_str, name, &law_data.law_body)
                    .iter()
                    .map(|result| {
                        let text = article::text_list_from_paragraph(&result.result)
                            .iter()
                            .map(|(_, text)| text.clone())
                            .collect::<Vec<String>>()
                            .join("\n");
                        LegalDocument::Law(Law {
                            id: id.clone(),
                            name: name.clone(),
                            index: result.article_index.clone(),
                            text,
                        })
                    })
                    .collect::<Vec<LegalDocument>>();
            info!("[END] parsing law: {id_str}");
            info!("[START] register law: {id_str}");
            legal_document_registory.add_data(&article_list).await?;
            info!("[END] register law: {id_str}");
        }
    }
    info!("[END] parsing law data");

    // 検索エンジン用の判例データを生成する
    info!("[START] parsing precedent data");
    let mut precedent_index_f = File::open(app_args.precedent_index).await?;
    let mut precedent_index_buf = Vec::new();
    precedent_index_f
        .read_to_end(&mut precedent_index_buf)
        .await?;
    let precedent_index_list = serde_json::from_slice::<Vec<PrecedentInfo>>(&precedent_index_buf)?;
    let mut precedent_index_stream = tokio_stream::iter(precedent_index_list);
    while let Some(precedent_info) = precedent_index_stream.next().await {
        let file_path = Path::new(&app_args.precedent_folder).join(precedent_info.file_name());
        let mut precedent_file = File::open(file_path).await?;
        let mut precedent_buf = Vec::new();
        precedent_file.read_to_end(&mut precedent_buf).await?;
        let precedent = serde_json::from_slice::<PrecedentData>(&precedent_buf)?;
        info!("[END] parsing precedent: {}", precedent_info.lawsuit_id);
        info!("[START] register precedent: {}", precedent_info.lawsuit_id);
        if let Some(text) = precedent.contents {
            let v = [LegalDocument::Precedent(Precedent {
                id: precedent_info.clone(),
                text,
            })];
            legal_document_registory.add_data(&v).await?;
        }
        info!("[END] register precedent: {}", precedent_info.lawsuit_id);
    }
    info!("[END] parsing precedent data");

    info!("all done");

    //TODO

    Ok(())
}
