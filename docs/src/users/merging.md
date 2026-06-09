# Merging Changes

Merging copies content from the left (old) side into the right (new) side.

## Apply a single hunk

Click the **▶** button in the action column of a changed hunk.  The right side
updates immediately; the hunk becomes equal and shows ✓.

## Undo

Click **Undo** (or reveal it via **More ▼**) to reverse the most recent apply.
Each apply is one undoable step; you can undo back to the original state.

## Save

When you are satisfied, click **Save**.  ForskScope:

1. Checks whether the right-side file changed on disk since it was loaded.
   If it did, you are asked to confirm the overwrite or cancel.
2. Creates a `.bak` sibling backup of the original file.
3. Writes the merged result atomically (temp file → rename).
4. Clears the unsaved marker on the tab.
