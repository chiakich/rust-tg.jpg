use anyhow::{anyhow, Result};
use log::{info, warn};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::sync::OnceLock;

pub mod bing;
pub mod ddg;
pub mod google;
pub mod serpapi;
pub mod serper;

pub(crate) const MAX_RESULTS: usize = 10;
const HEALTH_CHECK_QUERY: &str = "cat";
static ENABLED_ENGINES: OnceLock<Vec<SearchEngine>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SearchEngine {
  Serper,
  Google,
  SerpApi,
  Ddg,
  Bing,
}

impl SearchEngine {
  fn label(self) -> &'static str {
    match self {
      SearchEngine::Serper => "Serper",
      SearchEngine::Google => "Google",
      SearchEngine::SerpApi => "SerpAPI",
      SearchEngine::Ddg => "DDG",
      SearchEngine::Bing => "Bing",
    }
  }
}

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

pub async fn initialize() {
  if ENABLED_ENGINES.get().is_some() {
    return;
  }

  let serpapi_enabled = serpapi::is_configured();
  let serper_enabled = serper::is_configured();
  if serper_enabled {
    info!("SERPER_API detected; Serper image search will be health-checked.");
  } else {
    info!("SERPER_API not set; Serper image search is disabled.");
  }
  if serpapi_enabled {
    info!("SERP_API detected; SerpAPI image search will be health-checked.");
  } else {
    info!("SERP_API not set; SerpAPI image search is disabled.");
  }

  let (serper_result, google_result, serpapi_result, ddg_result, bing_result) = tokio::join!(
    run_optional_search(
      serper_enabled,
      SearchEngine::Serper,
      HEALTH_CHECK_QUERY,
      false
    ),
    google::search(HEALTH_CHECK_QUERY, false),
    run_optional_search(
      serpapi_enabled,
      SearchEngine::SerpApi,
      HEALTH_CHECK_QUERY,
      false
    ),
    ddg::search(HEALTH_CHECK_QUERY, false),
    bing::search(HEALTH_CHECK_QUERY, false)
  );

  let mut enabled = Vec::new();
  update_health_optional(&mut enabled, SearchEngine::Serper, serper_result);
  update_health(&mut enabled, SearchEngine::Google, google_result);
  update_health_optional(&mut enabled, SearchEngine::SerpApi, serpapi_result);
  update_health(&mut enabled, SearchEngine::Ddg, ddg_result);
  update_health(&mut enabled, SearchEngine::Bing, bing_result);

  if enabled.is_empty() {
    warn!("No image search engines passed the startup health check.");
  } else {
    info!(
      "Enabled image search engines: {}",
      enabled
        .iter()
        .map(|engine| engine.label())
        .collect::<Vec<_>>()
        .join(", ")
    );
  }

  let _ = ENABLED_ENGINES.set(enabled);
}

pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, anyhow::Error> {
  let enabled = ENABLED_ENGINES
    .get()
    .cloned()
    .unwrap_or_else(default_engines);

  let use_serper = enabled.contains(&SearchEngine::Serper);
  let use_google = enabled.contains(&SearchEngine::Google);
  let use_serpapi = enabled.contains(&SearchEngine::SerpApi);
  let use_ddg = enabled.contains(&SearchEngine::Ddg);
  let use_bing = enabled.contains(&SearchEngine::Bing);

  let (serper_result, google_result, serpapi_result, ddg_result, bing_result) = tokio::join!(
    run_optional_search(use_serper, SearchEngine::Serper, query, is_gif),
    run_optional_search(use_google, SearchEngine::Google, query, is_gif),
    run_optional_search(use_serpapi, SearchEngine::SerpApi, query, is_gif),
    run_optional_search(use_ddg, SearchEngine::Ddg, query, is_gif),
    run_optional_search(use_bing, SearchEngine::Bing, query, is_gif)
  );

  let mut combined = Vec::new();
  let mut seen = HashSet::new();
  let mut had_success = false;
  let mut errors = Vec::new();

  merge_results(
    "Serper",
    serper_result,
    &mut combined,
    &mut seen,
    &mut had_success,
    &mut errors,
  );
  merge_results(
    "Google",
    google_result,
    &mut combined,
    &mut seen,
    &mut had_success,
    &mut errors,
  );
  merge_results(
    "SerpAPI",
    serpapi_result,
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
        "All enabled search engines completed but returned no image URLs."
      ));
    }

    return Err(anyhow!(
      "All enabled search engines failed: {}",
      errors.join(" | ")
    ));
  }

  info!(
    "Combined image search returned {} URLs for query '{}'",
    combined.len(),
    query
  );
  Ok(combined)
}

fn merge_results(
  source: &str,
  result: Option<std::result::Result<Vec<String>, SearchError>>,
  combined: &mut Vec<String>,
  seen: &mut HashSet<String>,
  had_success: &mut bool,
  errors: &mut Vec<String>,
) {
  let Some(result) = result else {
    return;
  };

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
      warn!("{} image search failed: {}", source, err);
      errors.push(format!("{}: {}", source, err));
    }
  }
}

fn default_engines() -> Vec<SearchEngine> {
  let mut engines = vec![SearchEngine::Google, SearchEngine::Ddg, SearchEngine::Bing];
  if serper::is_configured() {
    engines.insert(0, SearchEngine::Serper);
  }
  if serpapi::is_configured() {
    let insert_index = if serper::is_configured() { 2 } else { 1 };
    engines.insert(insert_index, SearchEngine::SerpApi);
  }
  engines
}

async fn run_optional_search(
  enabled: bool,
  engine: SearchEngine,
  query: &str,
  is_gif: bool,
) -> Option<std::result::Result<Vec<String>, SearchError>> {
  if !enabled {
    return None;
  }

  Some(match engine {
    SearchEngine::Serper => serper::search(query, is_gif).await,
    SearchEngine::Google => google::search(query, is_gif).await,
    SearchEngine::SerpApi => serpapi::search(query, is_gif).await,
    SearchEngine::Ddg => ddg::search(query, is_gif).await,
    SearchEngine::Bing => bing::search(query, is_gif).await,
  })
}

fn update_health(
  enabled: &mut Vec<SearchEngine>,
  engine: SearchEngine,
  result: std::result::Result<Vec<String>, SearchError>,
) {
  match result {
    Ok(urls) if !urls.is_empty() => {
      info!("Health check passed: {}", engine.label());
      enabled.push(engine);
    }
    Ok(_) => warn!("Health check failed: {} returned no URLs", engine.label()),
    Err(err) => warn!("Health check failed: {} ({})", engine.label(), err),
  }
}

fn update_health_optional(
  enabled: &mut Vec<SearchEngine>,
  engine: SearchEngine,
  result: Option<std::result::Result<Vec<String>, SearchError>>,
) {
  if let Some(result) = result {
    update_health(enabled, engine, result);
  }
}
