use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const CONTAINER_DIRS: &[&str] = &["crates", "packages", "apps", "services", "modules"];
const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".primer",
    ".github",
    ".idea",
    ".vscode",
    ".next",
    ".turbo",
    "node_modules",
    "target",
    "dist",
    "build",
    "coverage",
    "vendor",
    "tmp",
    "temp",
    ".venv",
    "venv",
    "__pycache__",
];
const MAX_SCAN_DEPTH: usize = 4;
const MAX_SCAN_FILES: usize = 4000;
const MAX_CANDIDATES: usize = 3;
const MAX_SAMPLE_FILES: usize = 4;

#[derive(Debug, Clone, Serialize)]
pub struct RepositoryAnalysis {
    pub repository: String,
    pub goal: Option<String>,
    pub detected_languages: Vec<String>,
    pub scanned_files: usize,
    pub truncated: bool,
    pub candidates: Vec<Candidate>,
    pub general_risks: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub rank: usize,
    pub boundary: String,
    pub boundary_type: String,
    pub size: String,
    pub code_files: usize,
    pub test_files: usize,
    pub languages: Vec<String>,
    pub likely_files: Vec<String>,
    pub reason: String,
    pub verification_hint: String,
    pub risk_hints: Vec<String>,
    pub goal_match_terms: Vec<String>,
}

#[derive(Debug, Clone)]
struct BoundaryStats {
    path: PathBuf,
    boundary_type: BoundaryType,
    code_files: usize,
    test_files: usize,
    config_files: usize,
    sample_code_files: Vec<String>,
    sample_test_files: Vec<String>,
    languages: BTreeSet<String>,
    truncated: bool,
    goal_match_terms: Vec<String>,
    score: i32,
}

#[derive(Debug, Clone, Copy)]
enum BoundaryType {
    Directory,
    WorkspaceMember,
    RootFiles,
}

impl BoundaryType {
    fn as_str(self) -> &'static str {
        match self {
            BoundaryType::Directory => "directory",
            BoundaryType::WorkspaceMember => "workspace_member",
            BoundaryType::RootFiles => "root_files",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FileClass {
    Code(&'static str),
    Test(&'static str),
    Doc,
    Config,
    Other,
}

pub fn analyze_repository(repo_root: &Path, goal: Option<&str>) -> Result<RepositoryAnalysis> {
    let mut scan_state = ScanState::default();
    let mut candidates = collect_candidate_paths(repo_root)?
        .into_iter()
        .filter_map(|(path, boundary_type)| {
            let stats = analyze_boundary(repo_root, &path, boundary_type, &mut scan_state).ok()?;
            (stats.code_files > 0).then_some(stats)
        })
        .collect::<Vec<_>>();

    if let Some(root_files) = analyze_root_files(repo_root, &mut scan_state)? {
        candidates.push(root_files);
    }

    let goal_terms = goal_terms(goal);
    for stats in &mut candidates {
        stats.goal_match_terms = goal_matches(&goal_terms, stats);
        stats.score = score_candidate(stats);
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.code_files.cmp(&right.code_files))
            .then_with(|| left.path.cmp(&right.path))
    });

    let detected_languages = candidates
        .iter()
        .flat_map(|candidate| candidate.languages.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let top_candidates = candidates
        .into_iter()
        .take(MAX_CANDIDATES)
        .enumerate()
        .map(|(index, stats)| to_candidate(repo_root, stats, index + 1))
        .collect::<Vec<_>>();

    Ok(RepositoryAnalysis {
        repository: repo_root.display().to_string(),
        goal: goal.map(str::to_string),
        detected_languages,
        scanned_files: scan_state.scanned_files,
        truncated: scan_state.truncated,
        general_risks: general_risks(&top_candidates, goal, scan_state.truncated),
        recommendations: recommendations(&top_candidates),
        candidates: top_candidates,
    })
}

fn collect_candidate_paths(repo_root: &Path) -> Result<Vec<(PathBuf, BoundaryType)>> {
    let mut candidates = Vec::new();

    for entry in fs::read_dir(repo_root)
        .with_context(|| format!("failed to read {}", repo_root.display()))?
    {
        let entry = entry.with_context(|| "failed to read directory entry".to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };

        if !path.is_dir() || should_exclude_dir(name) {
            continue;
        }

        if CONTAINER_DIRS.contains(&name) {
            let mut added_child = false;
            for child in
                fs::read_dir(&path).with_context(|| format!("failed to read {}", path.display()))?
            {
                let child = child.with_context(|| "failed to read directory entry".to_string())?;
                let child_path = child.path();
                if !child_path.is_dir() {
                    continue;
                }
                candidates.push((child_path, BoundaryType::WorkspaceMember));
                added_child = true;
            }
            if !added_child {
                candidates.push((path, BoundaryType::Directory));
            }
            continue;
        }

        candidates.push((path, BoundaryType::Directory));
    }

    Ok(candidates)
}

fn analyze_boundary(
    repo_root: &Path,
    boundary_path: &Path,
    boundary_type: BoundaryType,
    scan_state: &mut ScanState,
) -> Result<BoundaryStats> {
    let mut stats = BoundaryStats {
        path: boundary_path.to_path_buf(),
        boundary_type,
        code_files: 0,
        test_files: 0,
        config_files: 0,
        sample_code_files: Vec::new(),
        sample_test_files: Vec::new(),
        languages: BTreeSet::new(),
        truncated: false,
        goal_match_terms: Vec::new(),
        score: 0,
    };
    visit_dir(
        repo_root,
        boundary_path,
        boundary_path,
        0,
        scan_state,
        &mut stats,
    )?;
    Ok(stats)
}

fn analyze_root_files(
    repo_root: &Path,
    scan_state: &mut ScanState,
) -> Result<Option<BoundaryStats>> {
    let mut stats = BoundaryStats {
        path: repo_root.to_path_buf(),
        boundary_type: BoundaryType::RootFiles,
        code_files: 0,
        test_files: 0,
        config_files: 0,
        sample_code_files: Vec::new(),
        sample_test_files: Vec::new(),
        languages: BTreeSet::new(),
        truncated: false,
        goal_match_terms: Vec::new(),
        score: 0,
    };

    for entry in fs::read_dir(repo_root)
        .with_context(|| format!("failed to read {}", repo_root.display()))?
    {
        let entry = entry.with_context(|| "failed to read directory entry".to_string())?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        register_file(repo_root, &path, scan_state, &mut stats);
    }

    Ok((stats.code_files > 0).then_some(stats))
}

fn visit_dir(
    repo_root: &Path,
    boundary_root: &Path,
    current_dir: &Path,
    depth: usize,
    scan_state: &mut ScanState,
    stats: &mut BoundaryStats,
) -> Result<()> {
    if depth > MAX_SCAN_DEPTH || scan_state.scanned_files >= MAX_SCAN_FILES {
        scan_state.truncated = true;
        stats.truncated = true;
        return Ok(());
    }

    for entry in fs::read_dir(current_dir)
        .with_context(|| format!("failed to read {}", current_dir.display()))?
    {
        let entry = entry.with_context(|| "failed to read directory entry".to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };

        if path.is_dir() {
            if path != boundary_root && should_exclude_dir(name) {
                continue;
            }
            visit_dir(
                repo_root,
                boundary_root,
                &path,
                depth + 1,
                scan_state,
                stats,
            )?;
            if scan_state.scanned_files >= MAX_SCAN_FILES {
                scan_state.truncated = true;
                stats.truncated = true;
                break;
            }
            continue;
        }

        register_file(repo_root, &path, scan_state, stats);
        if scan_state.scanned_files >= MAX_SCAN_FILES {
            scan_state.truncated = true;
            stats.truncated = true;
            break;
        }
    }

    Ok(())
}

fn register_file(
    repo_root: &Path,
    path: &Path,
    scan_state: &mut ScanState,
    stats: &mut BoundaryStats,
) {
    if scan_state.scanned_files >= MAX_SCAN_FILES {
        scan_state.truncated = true;
        stats.truncated = true;
        return;
    }

    scan_state.scanned_files += 1;
    match classify_file(repo_root, path) {
        FileClass::Code(language) => {
            stats.code_files += 1;
            stats.languages.insert(language.to_string());
            push_sample(
                &mut stats.sample_code_files,
                relative_display(repo_root, path),
            );
        }
        FileClass::Test(language) => {
            stats.code_files += 1;
            stats.test_files += 1;
            stats.languages.insert(language.to_string());
            let sample = relative_display(repo_root, path);
            push_sample(&mut stats.sample_code_files, sample.clone());
            push_sample(&mut stats.sample_test_files, sample);
        }
        FileClass::Config => {
            stats.config_files += 1;
        }
        FileClass::Doc | FileClass::Other => {}
    }
}

fn classify_file(repo_root: &Path, path: &Path) -> FileClass {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let relative = relative_display(repo_root, path).to_ascii_lowercase();

    if file_name.ends_with(".md") {
        return FileClass::Doc;
    }

    if is_test_path(&relative)
        && let Some(language) = code_language(path)
    {
        return FileClass::Test(language);
    }

    if let Some(language) = code_language(path) {
        return FileClass::Code(language);
    }

    if is_config_file(file_name) {
        return FileClass::Config;
    }

    FileClass::Other
}

fn code_language(path: &Path) -> Option<&'static str> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    match file_name {
        "Cargo.toml" => None,
        "Makefile" => Some("Make"),
        _ => match path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
        {
            "rs" => Some("Rust"),
            "py" => Some("Python"),
            "js" | "cjs" | "mjs" => Some("JavaScript"),
            "ts" | "tsx" => Some("TypeScript"),
            "jsx" => Some("JavaScript"),
            "go" => Some("Go"),
            "rb" => Some("Ruby"),
            "java" => Some("Java"),
            "kt" | "kts" => Some("Kotlin"),
            "c" | "h" => Some("C"),
            "cc" | "cpp" | "cxx" | "hpp" | "hh" => Some("C++"),
            "cs" => Some("C#"),
            "swift" => Some("Swift"),
            "php" => Some("PHP"),
            "sh" | "bash" | "zsh" => Some("Shell"),
            _ => None,
        },
    }
}

fn is_test_path(relative: &str) -> bool {
    relative.contains("/tests/")
        || relative.contains("/test/")
        || relative.contains("/spec/")
        || relative.ends_with("_test.rs")
        || relative.ends_with("_test.go")
        || relative.ends_with("_spec.rb")
        || relative.ends_with("_test.py")
        || relative.ends_with(".test.ts")
        || relative.ends_with(".test.tsx")
        || relative.ends_with(".test.js")
        || relative.ends_with(".spec.ts")
        || relative.ends_with(".spec.tsx")
        || relative.ends_with(".spec.js")
}

fn is_config_file(file_name: &str) -> bool {
    matches!(
        file_name,
        "Cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "go.mod"
            | "Gemfile"
            | "composer.json"
            | "tsconfig.json"
            | "vite.config.ts"
            | "vite.config.js"
            | "webpack.config.js"
            | "webpack.config.ts"
            | "Makefile"
    ) || matches!(
        Path::new(file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default(),
        "toml" | "json" | "yaml" | "yml"
    )
}

fn score_candidate(stats: &BoundaryStats) -> i32 {
    let mut score = match stats.code_files {
        1..=4 => 35,
        5..=10 => 25,
        11..=20 => 12,
        _ => 0,
    };

    if stats.test_files > 0 {
        score += 15;
    }
    if stats.config_files > 0 {
        score += 5;
    }
    if stats.boundary_type.is_workspace_member() {
        score += 10;
    }
    if stats.boundary_type.is_root_files() {
        score -= 5;
    }
    if stats.goal_match_terms.is_empty() {
        score -= 2;
    } else {
        score += (stats.goal_match_terms.len() as i32) * 12;
    }
    if stats.code_files > 20 {
        score -= 10;
    }
    if stats.languages.len() > 1 {
        score -= 5;
    }

    score
}

fn to_candidate(repo_root: &Path, stats: BoundaryStats, rank: usize) -> Candidate {
    let boundary = match stats.boundary_type {
        BoundaryType::RootFiles => "repository root files".to_string(),
        _ => relative_display(repo_root, &stats.path),
    };
    let size = match stats.code_files {
        0..=4 => "small",
        5..=12 => "medium",
        _ => "large",
    }
    .to_string();
    let reason = candidate_reason(&stats, &boundary);
    let verification_hint = verification_hint(&stats, &boundary);
    let risk_hints = candidate_risks(&stats);
    let likely_files = stats
        .sample_code_files
        .iter()
        .chain(stats.sample_test_files.iter())
        .take(MAX_SAMPLE_FILES)
        .cloned()
        .collect::<Vec<_>>();

    Candidate {
        rank,
        boundary,
        boundary_type: stats.boundary_type.as_str().to_string(),
        size,
        code_files: stats.code_files,
        test_files: stats.test_files,
        languages: stats.languages.into_iter().collect(),
        likely_files,
        reason,
        verification_hint,
        risk_hints,
        goal_match_terms: stats.goal_match_terms,
    }
}

fn candidate_reason(stats: &BoundaryStats, boundary: &str) -> String {
    if !stats.goal_match_terms.is_empty() {
        return format!(
            "Best keyword match for the stated goal inside {} while still keeping the first step bounded.",
            boundary
        );
    }
    if stats.test_files > 0 && stats.code_files <= 10 {
        return format!(
            "{} is a relatively small boundary with existing tests or verification files nearby.",
            boundary
        );
    }
    if stats.boundary_type.is_workspace_member() {
        return format!(
            "{} is a package-level boundary, which is usually safer than a repository-wide first change.",
            boundary
        );
    }
    if stats.code_files <= 4 {
        return format!(
            "{} is one of the smallest source boundaries in the repository.",
            boundary
        );
    }

    format!(
        "{} looks like a meaningful repo boundary, but the first milestone should stay focused on one observable change inside it.",
        boundary
    )
}

fn verification_hint(stats: &BoundaryStats, boundary: &str) -> String {
    if let Some(test_file) = stats.sample_test_files.first() {
        return format!(
            "Prefer a first milestone that changes one behavior in {} and keeps {} passing.",
            boundary, test_file
        );
    }
    if let Some(code_file) = stats.sample_code_files.first() {
        return format!(
            "Keep the first milestone to one observable change near {} and add a focused automated check before broader edits.",
            code_file
        );
    }

    format!(
        "Pick one behavior inside {} that can be verified with a short focused check before moving to adjacent modules.",
        boundary
    )
}

fn candidate_risks(stats: &BoundaryStats) -> Vec<String> {
    let mut risks = Vec::new();

    if stats.code_files > 12 {
        risks.push(format!(
            "This boundary already spans {} code files; split below this path if the first milestone still feels broad.",
            stats.code_files
        ));
    }
    if stats.test_files == 0 {
        risks.push(
            "No obvious tests or verification scripts were found in this boundary; the first milestone may need to add verification first."
                .to_string(),
        );
    }
    if stats.languages.len() > 1 {
        risks.push(format!(
            "This boundary mixes multiple languages ({}); keep the first milestone inside one toolchain if possible.",
            stats.languages.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    if stats.truncated {
        risks.push(
            "Repository scanning hit the analysis limit here; inspect this boundary manually before treating it as small."
                .to_string(),
        );
    }

    risks
}

fn general_risks(candidates: &[Candidate], goal: Option<&str>, truncated: bool) -> Vec<String> {
    let mut risks = Vec::new();

    if candidates.is_empty() {
        risks.push(
            "Primer did not find an obvious source boundary. Start from the executable entrypoint, a service directory, or the smallest package with real code."
                .to_string(),
        );
        return risks;
    }

    if truncated {
        risks.push(
            format!(
                "Repository analysis stopped after {} files. If the suggested boundaries still look broad, inspect one level deeper before authoring the first milestone.",
                MAX_SCAN_FILES
            ),
        );
    }
    if candidates.iter().all(|candidate| candidate.test_files == 0) {
        risks.push(
            "None of the top candidates have obvious tests nearby. Expect the first brownfield milestone to create or tighten verification before larger implementation work."
                .to_string(),
        );
    }
    if candidates.len() > 1
        && candidates
            .iter()
            .filter(|candidate| !candidate.goal_match_terms.is_empty())
            .count()
            > 1
    {
        risks.push(
            "Multiple boundaries match the goal. Keep the first milestone inside one boundary and defer cross-cutting integration."
                .to_string(),
        );
    }
    if goal.is_some()
        && candidates
            .iter()
            .all(|candidate| candidate.goal_match_terms.is_empty())
    {
        risks.push(
            "The goal text did not map cleanly to repository paths. Confirm the boundary manually before you scaffold the first milestone."
                .to_string(),
        );
    }

    risks
}

fn recommendations(candidates: &[Candidate]) -> Vec<String> {
    if candidates.is_empty() {
        return vec![
            "Choose one directory or package with real executable code before initializing a workstream."
                .to_string(),
            "Write the first milestone so it proves one observable repo-specific behavior."
                .to_string(),
        ];
    }

    let first = &candidates[0];
    let mut recommendations = vec![format!(
        "Start with {} and keep the first milestone scoped to one behavior plus one verification step.",
        first.boundary
    )];
    recommendations.push(
        "Avoid a first milestone that changes multiple packages, services, or entrypoints at once."
            .to_string(),
    );
    if first.test_files == 0 {
        recommendations.push(
            "Author or tighten a focused automated check as part of the first milestone instead of assuming existing verification."
                .to_string(),
        );
    } else {
        recommendations.push(
            "Reuse the nearest existing automated check if you can, rather than introducing a repo-wide verification step."
                .to_string(),
        );
    }

    recommendations
}

fn goal_terms(goal: Option<&str>) -> Vec<String> {
    let Some(goal) = goal else {
        return Vec::new();
    };

    goal.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|part| {
            let term = part.trim().to_ascii_lowercase();
            (term.len() >= 3 && !is_stopword(&term)).then_some(term)
        })
        .collect()
}

fn goal_matches(goal_terms: &[String], stats: &BoundaryStats) -> Vec<String> {
    if goal_terms.is_empty() {
        return Vec::new();
    }

    let searchable = format!(
        "{} {} {}",
        relative_display_parentless(&stats.path).to_ascii_lowercase(),
        stats.sample_code_files.join(" ").to_ascii_lowercase(),
        stats.sample_test_files.join(" ").to_ascii_lowercase()
    );

    goal_terms
        .iter()
        .filter(|term| searchable.contains(term.as_str()))
        .cloned()
        .collect()
}

fn should_exclude_dir(name: &str) -> bool {
    name.starts_with('.') || EXCLUDED_DIRS.contains(&name)
}

fn is_stopword(term: &str) -> bool {
    matches!(
        term,
        "the"
            | "and"
            | "for"
            | "with"
            | "from"
            | "into"
            | "that"
            | "this"
            | "your"
            | "repo"
            | "repository"
            | "first"
            | "safe"
            | "small"
            | "step"
            | "change"
            | "build"
            | "make"
    )
}

fn relative_display(repo_root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(repo_root).unwrap_or(path);
    relative_display_parentless(relative)
}

fn relative_display_parentless(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn push_sample(samples: &mut Vec<String>, value: String) {
    if samples.len() >= MAX_SAMPLE_FILES || samples.contains(&value) {
        return;
    }
    samples.push(value);
}

#[derive(Default)]
struct ScanState {
    scanned_files: usize,
    truncated: bool,
}

impl BoundaryType {
    fn is_workspace_member(self) -> bool {
        matches!(self, BoundaryType::WorkspaceMember)
    }

    fn is_root_files(self) -> bool {
        matches!(self, BoundaryType::RootFiles)
    }
}
