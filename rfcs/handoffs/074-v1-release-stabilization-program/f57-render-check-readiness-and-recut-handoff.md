# F57 Developer Handoff: Render-Check Readiness and the 0.167.0 Re-cut

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M4 close-out — `0.167.0` is M5's candidate and cannot be produced until this lands
**Register items.** F57
**Baseline.** `main` at `fb246bd`; failed release run `31706778085`

This handoff directs execution of one slice. It does not redefine RFC-074.

## 1. What happened

`0.167.0` was tagged. The release workflow ran: **Release gates**, **macOS
aarch64** and **Windows x86_64** all passed; **Linux x86_64** failed at the F34
rendering check; **Create GitHub Release** was skipped.

```text
FAIL: could not find the 'File comparison' landmark
```

**No release was created — not even a draft.** Nothing is published, so the tag
re-cuts cleanly under `release.md`'s immutability policy.

**The check is at fault, not the product.** Two of three platform builds
succeeded and the binary is fine. Your review 056 §9 named this exact gap as
unproven — the detection logic was demonstrated, the runner environment was
not. That was the honest limit and it is what broke.

The check also did its job in the way that matters: it blocked a release before
packaging rather than letting one through.

## 2. Diagnosis

```python
def find_app(name, timeout_s=APP_TIMEOUT_S):
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:      # ← polls
        …
        time.sleep(POLL_INTERVAL_S)

def find_by_role(node, role, limit=None):
    if node.get_role_name() == role: return node
    for i in range(node.get_child_count()):  # ← single traversal, no retry
        …
    return None
```

The check polls until the *application* registers on AT-SPI, then immediately
walks the tree once for the landmark. An app registers as soon as its window
exists; the WebView's DOM — and therefore the `landmark` role — only appears
after first paint.

The log confirms the mechanism rather than merely being consistent with it:
launch at `13:55:25`, failure at `13:55:28`. Three seconds into a thirty-second
budget, because `limit` only aborts a long traversal, it never retries. The
runner software-renders (`libEGL warning: DRI3 error: Could not get DRI3
device`), so first paint is far slower than on a developer machine.

## 3. Required property

> The check proceeds only once the rendered tree is **ready**, and fails on a
> timeout that it actually consumes.

"Ready" is deliberately not "the landmark exists." `collect_rows()` has the same
single-traversal shape, so a tree caught mid-render could yield a partial row
set — which would either fail confusingly ("found only N table rows") or, worse,
compare a subset and pass. The fixture's row count is known and pinned by a
corpus test; use it. Waiting for the expected shape is the difference between a
check that is slow-tolerant and one that is merely lucky.

State what you chose as the readiness condition and why.

## 4. Make it exercisable without a tag — do this first

Right now the only way to run this check is to push a tag, which means every fix
attempt costs a re-cut. That is a bad loop to iterate in and the reason to fix
it before touching the check itself.

Add a way to run the render check on demand against a release build — a
`workflow_dispatch` entry point that builds and runs it **without** creating a
release or touching a tag. A separate workflow is probably cleaner than
conditioning the release job, but that is yours to choose; the requirement is
that the failing path can be reproduced and a fix confirmed *before* any tag
moves.

This also gives F34 the property M4-B's other gates have: it can be demonstrated
failing and passing on demand, rather than only observed in production.

## 5. Falsifiability

Per M4-B's standing standard, with the §4 entry point:

1. **Reproduce the failure** — show the current code failing on a runner with
   the same `could not find the 'File comparison' landmark` message.
2. **Show the fix passing** on the same path.
3. **Show it still detects the defect it exists for** — reintroduce F32's
   `sr-only` placement and confirm the check goes red on a runner, not just
   locally. A readiness fix that accidentally made the check permissive would
   otherwise look identical to a working one.

(3) is the one that matters most. It is easy to "fix" a timing problem by
loosening an assertion.

## 6. The re-cut

Once §4 and §5 are done, on `main`:

1. Delete the remote tag `0.167.0`, then re-tag the corrected commit.
   `release.md` permits this precisely because nothing left draft — and note its
   caution that `gh release delete --cleanup-tag` removes the tag too; there is
   no release to delete here, so use `git push origin --delete 0.167.0`.
2. **Record the re-cut in `CHANGELOG.md`'s `0.167.0` entry**, as the policy
   requires. One line: the first tag failed at the rendering check before any
   artifact was published, and why.
3. Push the tag and let the workflow run to a **draft**. Do not publish —
   publication is the owner's action.

## 7. Constraints

- `0.165.0` and `0.166.0` are published and immutable. `0.167.0` is not
  published and may be re-cut.
- No dependency is added, removed, or version-changed. In particular **do not
  bump `dioxus-desktop`** — the libxdo fix is merged upstream but unreleased,
  and `0.8.0-alpha.1` is not an option during v1 stabilization. This candidate
  knowingly carries F44.
- No product behaviour changes. This is test/CI tooling and a tag move.
- Do not weaken the check to make it pass. If the readiness condition cannot be
  met reliably, report that instead — a check that cannot run on the runner is a
  finding, not something to soften.

## 8. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. the readiness condition chosen, and why (§3);
3. the on-demand entry point (§4) and how to use it;
4. **the three demonstrations from §5, with observed output** — especially (3);
5. changed files;
6. the re-cut: old and new tag commit, and the CHANGELOG line;
7. executed gates with observed output;
8. the release run's result, job by job, and the draft's artifact list;
9. unresolved issues and known limitations;
10. requested review focus.

## 9. After this

`0.167.0` exists as a draft and M5 can begin against those exact artifacts —
carrying F44 knowingly, per the owner's decision to proceed rather than wait on
the upstream release.
