use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir should be set"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir should be set"));

    println!("cargo:rerun-if-changed=recipes");
    println!("cargo:rerun-if-changed=adapters/_shared");

    let source = render_bundled_data(&manifest_dir);
    fs::write(out_dir.join("bundled_data.rs"), source).expect("failed to write bundled data");
}

fn render_bundled_data(manifest_dir: &Path) -> String {
    let recipes_dir = manifest_dir.join("recipes");
    let shared_dir = manifest_dir.join("adapters").join("_shared");

    let recipe_dirs = discover_recipe_dirs(&recipes_dir);
    let shared_files = collect_files(&shared_dir);

    let mut source = String::new();

    for recipe_dir in &recipe_dirs {
        let recipe_id = recipe_dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("recipe dir name should be utf-8");
        let const_name = format!("{}_ASSETS", sanitize_ident(recipe_id).to_uppercase());
        source.push_str(&format!("const {const_name}: &[BundledAsset] = &[\n"));
        for path in collect_files(recipe_dir) {
            let relative = path
                .strip_prefix(recipe_dir)
                .expect("recipe file should be under recipe dir");
            let absolute = path.to_string_lossy();
            let relative = relative.to_string_lossy().replace('\\', "/");
            let executable = is_executable(&path);
            source.push_str(&format!(
                "    BundledAsset {{ relative_path: {relative:?}, contents: include_str!({absolute:?}), executable: {executable} }},\n"
            ));
        }
        source.push_str("];\n\n");
    }

    source.push_str("const BUNDLED_RECIPES: &[BundledRecipe] = &[\n");
    for recipe_dir in &recipe_dirs {
        let recipe_id = recipe_dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("recipe dir name should be utf-8");
        let const_name = format!("{}_ASSETS", sanitize_ident(recipe_id).to_uppercase());
        source.push_str(&format!(
            "    BundledRecipe {{ id: {recipe_id:?}, location: \"bundled\", assets: {const_name} }},\n"
        ));
    }
    source.push_str("];\n\n");

    source.push_str("const SHARED_CONTRACTS: &[SharedContract] = &[\n");
    for path in shared_files {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("shared contract name should be utf-8");
        let absolute = path.to_string_lossy();
        source.push_str(&format!(
            "    SharedContract {{ filename: {filename:?}, contents: include_str!({absolute:?}) }},\n"
        ));
    }
    source.push_str("];\n");

    source
}

fn discover_recipe_dirs(recipes_dir: &Path) -> Vec<PathBuf> {
    let mut recipe_dirs = fs::read_dir(recipes_dir)
        .expect("failed to read recipes dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join("recipe.yaml").is_file())
        .collect::<Vec<_>>();
    recipe_dirs.sort();
    recipe_dirs
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_files(root, &mut files);
    files.sort();
    files
}

fn visit_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

fn sanitize_ident(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
