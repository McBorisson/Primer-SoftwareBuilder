use anyhow::{Context, Result, bail};
use comfy_table::Color;
use std::fs;
use std::path::Path;

use crate::recipe;
use crate::state;
use crate::ui;

pub fn run(primer_root: &Path, workspace_hint: &Path) -> Result<()> {
    let state = state::load_from_workspace(workspace_hint)?;
    let recipe = recipe::load_by_id(primer_root, &state.recipe_id)?;
    let milestone = recipe::resolve_initial_milestone(&recipe, Some(&state.milestone_id))?;

    if state.recipe_path != recipe.path {
        bail!(
            "workspace state points to {}, but resolved recipe is {}",
            state.recipe_path.display(),
            recipe.path.display()
        );
    }

    let explanation_path = recipe
        .path
        .join("milestones")
        .join(&milestone.id)
        .join("explanation.md");

    if !explanation_path.is_file() {
        bail!(
            "milestone explanation not found at {}",
            explanation_path.display()
        );
    }

    let explanation = fs::read_to_string(&explanation_path)
        .with_context(|| format!("failed to read {}", explanation_path.display()))?;

    ui::section("Primer explain");
    println!();
    ui::key_value_table(&[
        ui::KeyValueRow {
            key: "Recipe".to_string(),
            value: recipe.id.clone(),
            value_color: None,
        },
        ui::KeyValueRow {
            key: "Current milestone".to_string(),
            value: format!("{} ({})", milestone.id, milestone.title),
            value_color: None,
        },
        ui::KeyValueRow {
            key: "Explanation file".to_string(),
            value: explanation_path.display().to_string(),
            value_color: Some(Color::DarkGrey),
        },
    ]);
    println!();
    ui::print_markdown(explanation.trim_end());

    Ok(())
}
