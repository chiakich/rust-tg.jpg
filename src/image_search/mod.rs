use anyhow::{anyhow, Result};
use log::{info, warn};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

pub mod bing;
pub mod ddg;
pub mod google;

pub(crate) const MAX_RESULTS: usize = 10;

#[derive(Debug)]
pub enum SearchError {
  Blocked {
    engine: &'static str,
    details: String,
  },
  NoResults {
    engine: &'static str,
  },
  ParseFailed {
    engine: &'static str,
    details: String,
  },
  NetworkFailed {
    engine: &'static str,
    details: String,
  },
}

impl fmt::Display for SearchError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SearchError::Blocked { engine, details } => {
        write!(f, "{} blocked: {}", engine, details)
      }
      SearchError::NoResults { engine } => write!(f, "{} returned no results", engine),
      SearchError::ParseFailed { engine, details } => {
        write!(f, "{} parse failed: {}", engine, details)
      }
      SearchError::NetworkFailed { engine, details } => {
        write!(f, "{} network failed: {}", engine, details)
      }
    }
  }
}

impl Error for SearchError {}

pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, anyhow::Error> {
  let (google_result, ddg_result, bing_result) = tokio::join!(
    google::search(query, is_gif),
    ddg::search(query, is_gif),
    bing::search(query, is_gif)
  );

  let mut combined = Vec::new();
  let mut seen = HashSet::new();
  let mut had_success = false;
  let mut errors = Vec::new();

  merge_results(
    "Google",
    google_result,
    &mut combined,
    &mut seen,
    &mut had_success,
    &mut errors,
  );
  merge_results(
    "DDG",
    ddg_result,
    &mut combined,
    &mut seen,
    &mut had_success,
    &mut errors,
  );
  merge_results(
    "Bing",
    bing_result,
    &mut combined,
    &mut seen,
    &mut had_success,
    &mut errors,
  );

  if combined.is_empty() {
    if had_success {
      return Err(anyhow!(
        "All search engines completed but returned no image URLs."
      ));
    }

    return Err(anyhow!("All search engines failed: {}", errors.join(" | ")));
  }

  info!(
    "Combined image search returned {} URLs for query '{}' with priority Google -> DDG -> Bing",
    combined.len(),
    query
  );
  Ok(combined)
}

fn merge_results(
  source: &str,
  result: std::result::Result<Vec<String>, SearchError>,
  combined: &mut Vec<String>,
  seen: &mut HashSet<String>,
  had_success: &mut bool,
  errors: &mut Vec<String>,
) {
  match result {
    Ok(urls) => {
      *had_success = true;
      info!("{} returned {} URLs", source, urls.len());

      for url in urls {
        if combined.len() >= MAX_RESULTS {
          break;
        }

        if seen.insert(url.clone()) {
          combined.push(url);
        }
      }
    }
    Err(err) => {
      warn!("{} image search failed: {:?}", source, err);
      errors.push(format!("{}: {}", source, err));
    }
  }
}
