# FAQ

**Why can I not save after comparing two `.xlsx` files?**

Excel comparison is read-only in ForskScope.  The content shown is derived from
the spreadsheet diff, not a direct representation of the file; saving that
derived text over a `.xlsx` would corrupt the file.  Future versions may add a
first-class Excel merge mode.

**Why is merge unavailable for a binary file?**

Binary files are compared via a hex preview.  There is no safe way to
interpret the diff as a meaningful edit to the original binary content.

**Can I compare files on a network drive?**

Yes — mount the drive through the OS as usual, then open ForskScope and
navigate to the mounted path.  ForskScope works with the local filesystem only
and does not connect to remote services itself.

**How do I undo all applied changes?**

Click **Undo** repeatedly.  Each click reverses one apply.  Undo / Redo are in
the **More ▼** disclosure on the toolbar.
