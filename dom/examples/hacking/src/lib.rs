use std::{cell::RefCell, rc::Rc};

use futures::{StreamExt, future::ready};
use mox::mox;
use moxie_dom::{
    elements::{
        forms::button,
        text_content::{div, Div},
    },
    prelude::*,
};
use moxie_streams::mox_stream;
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

#[derive(Clone)]
enum Msg {
    Increment,
    Decrement,
    Double,
}

#[topo::nested]
fn root() -> Div {
    let (ct, dispatch) = mox_stream(
        Rc::new(RefCell::new(0)),
        |state, msg| match msg {
            Msg::Increment => {
                *(state.borrow_mut()) += 1;
                state.clone()
            },
            Msg::Decrement => {
                *(state.borrow_mut()) -= 1;
                state.clone()
            },
            Msg::Double => {
                *(state.borrow_mut()) *= 2;
                state.clone()
            }
        },
        |stream| stream.filter(|msg| match msg {
            Msg::Increment => ready(true),
            _ => ready(false)
        }).map(|_| Msg::Double),
    );
    let d1 = dispatch.clone();
    let d2 = dispatch.clone();

    let mut root = div();

    root = root.child(mox! { <div>{% "hello world from moxie! ({})", ct.borrow() }</div> });
    root = root.child(mox! {
        <button type="button" onclick={move |_| {d1(Msg::Increment);}}>
            "increment"
        </button>
    });
    root = root.child(mox! {
        <button type="button" onclick={move |_| {d2(Msg::Decrement);}}>
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
