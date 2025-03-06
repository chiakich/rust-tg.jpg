use anyhow::Result;
use log::{error, info};
use std::fs;
use std::path::{Path, PathBuf};

// Define the assets directory
pub const ASSETS_DIR: &str = "src/assets";

// Find a matching image in the local assets directory
pub async fn find_local_image(text: &str) -> Result<Option<PathBuf>, anyhow::Error> {
  let assets_dir = Path::new(ASSETS_DIR);

  // Check if assets directory exists
  if !assets_dir.exists() {
    error!("Assets directory not found: {}", ASSETS_DIR);
    return Ok(None);
  }

  // Normalize the input text for fuzzy matching
  let normalized_text = normalize_text(text);

  // Store potential matches with their scores
  let mut matches: Vec<(PathBuf, usize)> = Vec::new();

  // Collect all image files from the assets directory and its subdirectories
  collect_potential_matches(assets_dir, &normalized_text, &mut matches)?;

  // Sort matches by score (highest first)
  matches.sort_by(|a, b| b.1.cmp(&a.1));

  // Return the best match if any
  if let Some((best_match, score)) = matches.first() {
    info!("Found fuzzy match with score {}: {:?}", score, best_match);
    return Ok(Some(best_match.clone()));
  }

  Ok(None)
}

// Find all matching images for inline query results
pub async fn find_matching_images(text: &str) -> Result<Vec<(PathBuf, usize)>, anyhow::Error> {
  let assets_dir = Path::new(ASSETS_DIR);

  // Check if assets directory exists
  if !assets_dir.exists() {
    error!("Assets directory not found: {}", ASSETS_DIR);
    return Ok(Vec::new());
  }

  // Normalize the input text for fuzzy matching
  let normalized_text = normalize_text(text);

  // Store potential matches with their scores
  let mut matches: Vec<(PathBuf, usize)> = Vec::new();

  // Collect all image files from the assets directory and its subdirectories
  collect_potential_matches(assets_dir, &normalized_text, &mut matches)?;

  // Sort matches by score (highest first)
  matches.sort_by(|a, b| b.1.cmp(&a.1));

  // Return all matches
  Ok(matches)
}

// Helper function to collect potential matches from the assets directory
fn collect_potential_matches(
  dir: &Path,
  normalized_text: &str,
  matches: &mut Vec<(PathBuf, usize)>,
) -> Result<(), anyhow::Error> {
  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();

    if path.is_dir() {
      // Recursively process subdirectories
      collect_potential_matches(&path, normalized_text, matches)?;
    } else if path.is_file() {
      process_file(&path, normalized_text, matches);
    }
  }

  Ok(())
}

// Process a single file to check if it matches the search text
fn process_file(file_path: &Path, normalized_text: &str, matches: &mut Vec<(PathBuf, usize)>) {
  // Get the file name without extension
  let file_stem = match file_path.file_stem().and_then(|s| s.to_str()) {
    Some(stem) if !stem.is_empty() => stem,
    _ => return, // Skip files with no valid stem
  };

  // Normalize the file name for matching
  let normalized_file_stem = normalize_text(file_stem);

  // Special handling for short file names (less than 3 characters)
  if normalized_file_stem.chars().count() < 3 {
    handle_short_filename(file_path, normalized_text, &normalized_file_stem, matches);
  } else {
    handle_normal_filename(file_path, normalized_text, &normalized_file_stem, matches);
  }
}

// Handle short filenames (less than 3 characters)
fn handle_short_filename(
  file_path: &Path,
  normalized_text: &str,
  normalized_file_stem: &str,
  matches: &mut Vec<(PathBuf, usize)>,
) {
  // For short file names, require exact match with the entire input text
  if normalized_text == normalized_file_stem {
    // Give a very high score for exact matches of short file names
    matches.push((file_path.to_path_buf(), 2000));
  }
}

// Handle normal length filenames (3 or more characters)
fn handle_normal_filename(
  file_path: &Path,
  normalized_text: &str,
  normalized_file_stem: &str,
  matches: &mut Vec<(PathBuf, usize)>,
) {
  // Check for containment match
  if normalized_text.contains(normalized_file_stem)
    || normalized_file_stem.contains(normalized_text)
  {
    // Calculate match score (higher is better)
    let score = calculate_match_score(normalized_text, normalized_file_stem);
    matches.push((file_path.to_path_buf(), score));
    return;
  }

  // Try fuzzy matching if no containment match
  let file_words: Vec<&str> = normalized_file_stem.split_whitespace().collect();
  let text_words: Vec<&str> = normalized_text.split_whitespace().collect();

  let mut word_matches = 0;
  for file_word in &file_words {
    if text_words
      .iter()
      .any(|&text_word| text_word.contains(file_word) || file_word.contains(text_word))
    {
      word_matches += 1;
    }
  }

  // If we have at least one word match
  if word_matches > 0 {
    // Calculate score based on percentage of words matched
    let score = (word_matches * 100) / file_words.len().max(1);
    matches.push((file_path.to_path_buf(), score));
  }
}

// Helper function to normalize text for better matching
fn normalize_text(text: &str) -> String {
  text
    .to_lowercase()
    .chars()
    .filter(|c| c.is_alphanumeric() || c.is_whitespace())
    .collect::<String>()
    .split_whitespace()
    .collect::<Vec<&str>>()
    .join(" ")
}

// Calculate a match score between two strings
fn calculate_match_score(text: &str, file_name: &str) -> usize {
  // If one contains the other completely, give a high score
  if text.contains(file_name) {
    return 1000 + file_name.len();
  }
  if file_name.contains(text) {
    return 900 + text.len();
  }

  // Count matching words
  let text_words: Vec<&str> = text.split_whitespace().collect();
  let file_words: Vec<&str> = file_name.split_whitespace().collect();

  let mut score = 0;
  for text_word in &text_words {
    for file_word in &file_words {
      if text_word == file_word {
        score += 100;
      } else if text_word.contains(file_word) || file_word.contains(text_word) {
        score += 50;
      }
    }
  }

  score
}
