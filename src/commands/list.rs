use crate::recipe;
use crate::ui;
use anyhow::Result;

pub fn run(source: &recipe::RecipeSource) -> Result<()> {
    let spinner = ui::spinner("Scanning Primer recipes...");
    let recipes = recipe::discover(source)?;
    spinner.finish_and_clear();

    if recipes.is_empty() {
        ui::info("No recipes found.");
        return Ok(());
    }

    ui::display_recipe_table(&recipes);

    Ok(())
}
