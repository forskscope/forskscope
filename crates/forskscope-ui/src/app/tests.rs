//! F61 regression test (handoff:
//! `rfcs/handoffs/074-v1-release-stabilization-program/f61-f62-session-persistence-handoff.md`).
//!
//! `with_test_store` (F36, M4-B) cannot reach this: it renders a synthetic
//! `root()` component that only constructs and captures a `Store`, never
//! `App()` itself — so `app.rs`'s reactive `use_effect` on `store.tabs`
//! (the thing F61 is actually about) never runs under it. This test renders
//! the *real* `App()` component instead, through a real `VirtualDom`, with a
//! genuine CLI `Compare` startup request — the exact path
//! `forskscope <left> <right>` takes — and asserts the session was actually
//! written, not merely that in-memory state looks right.
//!
//! `VirtualDom::rebuild_in_place()` runs `App()`'s `use_hook` (which opens
//! the startup tab synchronously — `open_compare_request` pushes onto
//! `store.tabs` before returning) but deliberately does not poll tasks or
//! run effects (its own doc comment: "Tasks will not be polled with this
//! method"). `render_immediate` is what actually pops and runs the queued
//! `use_effect`, synchronously. The tab-opening call also spawns a
//! background load/diff task (`spawn_forever`) that this test never drives
//! to completion — irrelevant here, since `save_session`'s payload only
//! needs `left_path`/`right_path`, both set before the async task even
//! starts. `render_immediate` still touches Dioxus's task-polling machinery
//! in general, which is why this runs inside an entered (not necessarily
//! driven) Tokio runtime rather than a bare `#[test]`.
//!
//! Also serializes on `state::DIOXUS_VDOM_TEST_LOCK`, shared with
//! `with_test_store`, against genuine concurrent-`VirtualDom` interference.
//! The same lock also happens to cover this test's `XDG_CONFIG_HOME`
//! mutation — nothing else in the suite touches that env var, but sharing
//! one lock is simpler than two.
//!
//! **`#[ignore]`d - a real, unresolved test-infrastructure limitation, not
//! a weaker substitute (handoff §3: "say so explicitly ... that is itself a
//! finding about F36's harness").** Empirically narrowed while diagnosing
//! F61 (`cargo test -p forskscope-ui --lib -- --test-threads=N`, run
//! repeatedly at each N): passes reliably (5/5) at `N<=2`; fails
//! reliably (5/5, real 10s timeout - the tabs-persist effect never runs at
//! all, not a slow pass) at every `N>=3` tried (3, 4, 8, 16, and the
//! ~32-thread default on this 32-core machine), regardless of which or how
//! many *other* tests are selected to run alongside it - even alone under
//! `--test-threads=2` with zero other tests executing, it still fails.
//! `DIOXUS_VDOM_TEST_LOCK` was added specifically to rule out cross-test
//! `VirtualDom` interference as the cause and did not fix it, so the
//! remaining cause is something else about `dioxus-core`/`tokio` under
//! heavier concurrent-thread load that this investigation did not run down
//! further. A related but distinct bug *was* found and fixed during this
//! same investigation: an earlier revision read `CAPTURED_STORE`'s signals
//! after dropping the `VirtualDom` that owned them, panicking with
//! `Dropped(ValueDroppedError)` - that ordering bug is fixed (see the
//! `toast` read inside `rt.block_on` below), and is not what `--test-
//! threads`-dependence is about.
//!
//! Run on demand: `cargo test -p forskscope-ui --lib -- --ignored
//! --test-threads=2 a_cli_opened_tab_is_persisted_to_session_json`.

use std::cell::RefCell;
use std::fs;

use dioxus::prelude::ReadableExt;
use dioxus::prelude::*;
use forskscope_core::persist::schema::PersistenceLoad;
use forskscope_core::persist::schema::session::SessionRepository;
use forskscope_ui_logic::StartupRequest;

use super::{App, STARTUP_REQUEST};
use crate::state::{DIOXUS_VDOM_TEST_LOCK, Store};

thread_local! {
    /// `App()`'s own `Store`, captured for inspection after a render — the
    /// same technique `with_test_store` uses, applied to the real component
    /// this time instead of a synthetic one.
    pub(super) static CAPTURED_STORE: RefCell<Option<Store>> = const { RefCell::new(None) };
}

#[test]
#[ignore = "reliable only at --test-threads<=2; see module docs"]
fn a_cli_opened_tab_is_persisted_to_session_json() {
    let _guard = DIOXUS_VDOM_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let scratch = std::env::temp_dir().join(format!(
        "fsk-ui-app-f61-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&scratch).unwrap();
    let left = scratch.join("left.txt");
    let right = scratch.join("right.txt");
    fs::write(&left, "alpha\n").unwrap();
    fs::write(&right, "beta\n").unwrap();
    let config_home = scratch.join("config");
    fs::create_dir_all(&config_home).unwrap();

    let previous_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    // SAFETY: serialized by DIOXUS_VDOM_TEST_LOCK; nothing else in this
    // suite reads XDG_CONFIG_HOME concurrently.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &config_home);
    }

    // OnceLock: settable once per process. This is the only test in the
    // suite that renders App(), so this is the only setter.
    let _ = STARTUP_REQUEST.set(StartupRequest::Compare {
        left: left.clone(),
        right: right.clone(),
    });

    let session_path = config_home.join("forskscope").join("session.json");

    // Drives Dioxus's own real scheduling loop (wait_for_work) instead of
    // manually re-invoking render_immediate - the same primitive a real
    // desktop launch's event loop uses, bounded so a genuine failure to
    // persist doesn't hang the test suite.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let toast = rt.block_on(async {
        let mut vdom = VirtualDom::new(App);
        vdom.rebuild_in_place();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        while !session_path.exists() && tokio::time::Instant::now() < deadline {
            let _ =
                tokio::time::timeout(std::time::Duration::from_millis(100), vdom.wait_for_work())
                    .await;
            vdom.render_immediate(&mut dioxus::core::NoOpMutations);
        }
        // Must read the captured Store's signals before dropping vdom -
        // they live in its root scope and become unreadable once it's
        // gone (confirmed the hard way: reading toast after an earlier
        // version's premature `drop(vdom)` panicked with
        // `Dropped(ValueDroppedError)`, not a Dioxus concurrency issue at
        // all despite how it first looked).
        let captured = CAPTURED_STORE.with(|c| c.borrow_mut().take()).expect(
            "App()'s use_context_provider must have run and captured its \
             Store during rebuild_in_place()",
        );
        let toast = captured.toast.read().clone();
        drop(vdom);
        toast
    });
    assert_eq!(
        toast, None,
        "the session save must succeed silently, not report an error toast"
    );

    drop(rt);

    // SAFETY: still serialized by DIOXUS_VDOM_TEST_LOCK.
    unsafe {
        match &previous_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    assert!(
        session_path.exists(),
        "F61: a CLI-opened tab (forskscope {} {}) must be persisted to \
         session.json, but no file was written at {}",
        left.display(),
        right.display(),
        session_path.display()
    );

    let repo = SessionRepository::new(session_path.clone());
    match repo.load() {
        PersistenceLoad::Current { value } => {
            assert_eq!(
                value.tabs.len(),
                1,
                "expected exactly the one CLI-opened tab, got {:?}",
                value.tabs
            );
            assert_eq!(value.tabs[0].left, left);
            assert_eq!(value.tabs[0].right, right);
        }
        other => panic!(
            "session.json was written but does not parse as a current-schema \
             session: {other:?}"
        ),
    }

    let _ = fs::remove_dir_all(&scratch);
}
