use leptos::*;

use similar::DiffTag;

#[component]
pub fn DiffBlock(diff_tag: DiffTag, source: Vec<String>, target: Vec<String>, is_target: bool, on_input: impl Fn(leptos::ev::Event) + 'static + Copy, tab_index: usize) -> impl IntoView {
    // use web_sys::console;
    // console::log_1(&(if source == "" {wasm_bindgen::JsValue::from_str("y")} else {wasm_bindgen::JsValue::from_str("n")} ));

    match diff_tag {
        DiffTag::Replace => {
            view! {
                <For
                    each=move || (if is_target { target.clone() } else { source.clone() }).into_iter().enumerate()
                    key=|n| n.clone()
                    children=move |(_, x)| {
                        view! {
                            <pre on:input=on_input style={if is_target { "color: green;" } else { "color: red;" } } contenteditable tabindex={tab_index}>
                                { x }
                            </pre>
                        }           
                    } />
            }
        },
        DiffTag::Delete => {
            view! {
                <For
                    each=move || (if is_target { std::iter::repeat("".to_owned()).take(source.len()).collect() } else { source.clone() }).into_iter().enumerate()
                    key=|n| n.clone()
                    children=move |(_, x)| {
                        view! {
                            <pre on:input=on_input style={if is_target { "color: green;" } else { "color: red;" } } contenteditable tabindex={tab_index}>
                                { x }
                            </pre>
                        }           
                    } />
            }
        },
        DiffTag::Insert => {
            view! {
                <For
                    each=move || (if is_target { target.clone() } else { std::iter::repeat("".to_owned()).take(target.len()).collect() }).into_iter().enumerate()
                    key=|n| n.clone()
                    children=move |(_, x)| {
                        view! {
                            <pre on:input=on_input style={if is_target { "color: green;" } else { "color: red;" } } contenteditable tabindex={tab_index}>
                                { x }
                            </pre>
                        }           
                    } />
            }
        },
        DiffTag::Equal => {
            view! {
                <For
                    each=move || target.clone().into_iter().enumerate()
                    key=|n| n.clone()
                    children=move |(_, x)| {
                        view! {
                            <pre on:input=on_input style="color: silver;" contenteditable tabindex={tab_index}>
                                { x }
                            </pre>
                        }           
                    } />
            }
        },
    }
}