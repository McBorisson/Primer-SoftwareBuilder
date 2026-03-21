use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

pub struct BundledAsset {
    pub relative_path: &'static str,
    pub contents: &'static str,
    pub executable: bool,
}

pub struct BundledRecipe {
    pub id: &'static str,
    pub location: &'static str,
    pub assets: &'static [BundledAsset],
}

pub struct SharedContract {
    pub filename: &'static str,
    pub contents: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/bundled_data.rs"));

pub fn recipes() -> &'static [BundledRecipe] {
    BUNDLED_RECIPES
}

pub fn recipe(id: &str) -> Option<&'static BundledRecipe> {
    BUNDLED_RECIPES.iter().find(|recipe| recipe.id == id)
}

pub fn shared_contract(filename: &str) -> Option<&'static str> {
    SHARED_CONTRACTS
        .iter()
        .find(|contract| contract.filename == filename)
        .map(|contract| contract.contents)
}

pub fn materialize_recipe(recipe_id: &str, recipe_dir: &Path) -> Result<PathBuf> {
    let recipe =
        recipe(recipe_id).ok_or_else(|| anyhow!("unknown bundled recipe '{recipe_id}'"))?;
    write_recipe(recipe, recipe_dir)?;
    Ok(recipe_dir.to_path_buf())
}

fn write_recipe(recipe: &BundledRecipe, recipe_dir: &Path) -> Result<()> {
    for asset in recipe.assets {
        let path = recipe_dir.join(asset.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, asset.contents)?;
        set_permissions(&path, asset.executable)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_permissions(path: &Path, executable: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mode = if executable { 0o755 } else { 0o644 };
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions(_path: &Path, _executable: bool) -> Result<()> {
    Ok(())
}

pub fn require_shared_contract(filename: &str) -> Result<&'static str> {
    shared_contract(filename).ok_or_else(|| anyhow!("missing bundled shared contract '{filename}'"))
}

pub fn require_recipe(id: &str) -> Result<&'static BundledRecipe> {
    recipe(id).ok_or_else(|| anyhow!("unknown bundled recipe '{id}'"))
}
