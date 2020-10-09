use futures::future::ready;
use futures_signals::signal::{Mutable, SignalExt};
use mox::mox;
use moxie_dom::{
    elements::{
        forms::button,
        text_content::{div, Div},
    },
    prelude::*,
};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn begin() {
    console_log::init_with_level(tracing::log::Level::Debug).unwrap();
    std::panic::set_hook(Box::new(|info| {
        tracing::error!("{:#?}", info);
    }));

    tracing::info!("mounting moxie-dom to root!!");
    moxie_dom::embed::WebRuntime::new(document().body().unwrap(), root)
        .animation_frame_scheduler()
        .run_on_wake();
}

enum Msg {
    Increment,
    Decrement,
}

fn state_signal<T: 'static + Clone, U>(
    update: impl Fn(T, U) -> T,
    initial_state: T,
) -> (Commit<T>, Rc<impl Fn(U)>) {
    let (current_state, state) = state(|| initial_state.clone());
    let m = once(|| Mutable::new(initial_state.clone()));
    let _ = load_once(|| {
        m.signal_cloned().for_each(move |v| {
            state.update(|_| Some(v));
            ready(())
        })
    });

    (current_state, Rc::new(move |msg: U| m.set(update(m.get_cloned(), msg))))
}

#[topo::nested]
fn root() -> Div {
    let (ct, dispatch) = state_signal(
        |state, msg| match msg {
            Msg::Increment => state + 1,
            Msg::Decrement => state - 1,
        },
        0,
    );
    let d1 = dispatch.clone();
    let d2 = dispatch.clone();

    let mut root = div();

    root = root.child(mox! { <div>{% "hello world from moxie! ({})", &ct }</div> });
    root = root.child(mox! {
        <button type="button" onclick={move |_| d1(Msg::Increment)}>
            "increment"
        </button>
    });
    root = root.child(mox! {
        <button type="button" onclick={move |_| d2(Msg::Decrement)}>
            "decrement"
        </button>
    });

    for t in &["first", "second", "third"] {
        root = root.child(mox! { <div>{% "{}", t }</div> });
    }

    root.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use augdom::{event::Click, testing::Query};
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    pub async fn hello_browser() {
        let test_root = augdom::Node::new("div");
        moxie_dom::boot(test_root.clone(), root);

        let button = test_root.find().by_text("increment").until().one().await.unwrap();
        assert_eq!(
            test_root.first_child().unwrap().to_string(),
            r#"<div>
  <div>hello world from moxie! (0)</div>
  <button type="button">increment</button>
  <div>first</div>
  <div>second</div>
  <div>third</div>
</div>"#
        );

        button.dispatch::<Click>();
        test_root.find().by_text("hello world from moxie! (1)").until().one().await.unwrap();
        button.dispatch::<Click>();
        test_root.find().by_text("hello world from moxie! (2)").until().one().await.unwrap();
    }
}
