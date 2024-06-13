use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use similar::DiffTag;
use std::ops::Range;

use crate::ui::diff_block::DiffBlock;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct GreetArgs<'a> {
    name: &'a str,
}

#[derive(Clone, Eq, Hash)]
struct DiffBlockOp {
    tag: DiffTag,
    old: Range<usize>,
    new: Range<usize>,
}
impl PartialEq for DiffBlockOp {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.old == other.old && self.new == other.new
    }
}

#[component]
pub fn App() -> impl IntoView {
    // const S: &str = "Wow This\nis perhaps a\ncat.\n";
    // const T: &str = "This\nis not a\ncat.\nOh...\n";
    // let diff_words_opts = TextDiff::configure().diff_chars(S, T).ops().to_owned();
    // let diff_words_opts2 = diff_words_opts.clone();
    const S: &str = "Wow This\n\nis perhaps a\ncat.\nNow time\nto go\noutside ?\n";
    const T: &str = "This\nis not a\ncat.\nOh...\nNow time\nto go\nabroad!!\nYay\n\n";
    let s = S.split("\n").collect::<Vec<&str>>();
    let t = T.split("\n").collect::<Vec<&str>>();
    let d = similar::TextDiff::configure().diff_slices(s.as_slice(), t.as_slice());
    let ops: Vec<DiffBlockOp> = d.ops().iter().map(|x| DiffBlockOp{ tag: x.tag(), old: x.old_range(), new: x.new_range()}).collect::<Vec<_>>();
    // d.ops().iter().for_each(|x| println!("{:?}|{}|{}", x.tag(), s[x.old_range()].join("\n"), t[x.new_range()].join("\n")));

    // use web_sys::console;
    // let t = similar::utils::diff_lines(similar::Algorithm::Myers, S, T).
    // console::log_1

    // let (ops, set_ops) = create_signal(Vec::<DiffBlockOp>::new());
    let (ops, set_ops) = create_signal(ops);

    view! {
        <main class="container">
            <div style="width: 96vw;">
                <For
                    each=move || ops.get().into_iter().enumerate()
                    key=|n| n.clone()
                    children=move |(n, x)| {
                        let source = s[x.old].iter().map(|x| x.to_string()).collect::<Vec<String>>();
                        let target = t[x.new].iter().map(|x| x.to_string()).collect::<Vec<String>>();

                        view! {
                            <div style="display: flex;" tabindex="-1" class={ format!("diff diff-{}", n) }>
                                <div style="display: block; width: 48vw;" tabindex="-1">
                                    <DiffBlock diff_tag=x.tag source=source.clone() target=target.clone() is_target=false on_input={move |ev: leptos::ev::Event| {
                                        let target = ev.target().unwrap();
                                        let target: &web_sys::HtmlElement = target.dyn_ref().unwrap();                                    target.inner_text();
                                        // todo use innerText
                                        let _inner_text = target.inner_text();
                                        set_ops.set(ops.get_untracked().iter().map(|x| if x.tag != DiffTag::Equal {x.to_owned()} else {DiffBlockOp {tag: DiffTag::Replace, old: x.old.clone(), new: x.new.clone()} } ).collect::<Vec<DiffBlockOp>>());
                                    }} tab_index=n />
                                </div>
                                <div style="display: block; width: 48vw;" tabindex="-1">
                                    <DiffBlock diff_tag=x.tag source=source.clone() target=target.clone() is_target=true on_input=move |_| { println!("test"); }  tab_index=1000 + n />
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </main>
    }
}
