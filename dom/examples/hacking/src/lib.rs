use std::{cell::RefCell, rc::Rc};

use futures::{channel::mpsc, future::ready, join, SinkExt, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};
use mox::mox;
use moxie_dom::{
    elements::{
        forms::button,
        text_content::{div, Div},
    },
    prelude::*,
};
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

async fn foo<T: Clone, Msg>(
    sender: mpsc::Sender<Msg>,
    receiver: mpsc::Receiver<Msg>,
    m: Mutable<T>,
    update: impl Fn(T, Msg) -> T + 'static,
) -> (Rc<RefCell<mpsc::Sender<Msg>>>, ()) {
    join!(
        ready(Rc::new(RefCell::new(sender))),
        receiver.for_each(move |msg| {
            m.set(update(m.get_cloned(), msg));
            ready(())
        })
    )
}

fn state_signal<T: 'static + Clone, Msg: 'static>(
    update: impl Fn(T, Msg) -> T + 'static,
    initial_state: T,
    buffer: usize,
) -> (Commit<T>, Rc<impl Fn(Msg)>) {
    let (current_state, x) = state(|| initial_state.clone());
    let (channel, _) = state(|| {
        let (s, r) = mpsc::channel(buffer) as (mpsc::Sender<Msg>, mpsc::Receiver<Msg>);
        (Rc::new(RefCell::new(s)), r)
    });
    let m = once(|| Mutable::new(initial_state.clone()));

    let _ = load_once(|| {
        m.signal_cloned().for_each(move |v| {
            x.update(|_| Some(v));
            ready(())
        })
    });
    let _ = load_once(|| {
        channel.1.for_each(move |msg| {
            m.set(update(m.get_cloned(), msg));
            ready(())
        })
    });

    let dispatch = once(|| {
        Rc::new(move |msg: Msg| {
            let _ = channel.0.borrow_mut().start_send(msg);
        })
    });

    (current_state, dispatch)
}

#[topo::nested]
fn root() -> Div {
    let (ct, dispatch) = state_signal(
        |state, msg| match msg {
            Msg::Increment => state + 1,
            Msg::Decrement => state - 1,
        },
        0,
        20,
    );
    let d1 = dispatch.clone();
    let d2 = dispatch.clone();

    let mut root = div();

    root = root.child(mox! { <div>{% "hello world from moxie! ({})", &ct }</div> });
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
