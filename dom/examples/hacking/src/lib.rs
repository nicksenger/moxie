use std::{cell::RefCell, rc::Rc};

use futures::{
    channel::mpsc,
    future::{err, ready},
    stream::Map,
    Stream, StreamExt,
};
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

#[derive(Clone)]
enum Msg {
    Increment,
    Decrement,
}

fn state_signal<State: 'static, Msg: 'static + Clone, OutStream>(
    initial_state: State,
    update: impl Fn(&State, Msg) -> State + 'static,
    operator: impl FnOnce(mpsc::UnboundedReceiver<Msg>) -> OutStream,
) -> (Commit<State>, Rc<impl Fn(Msg)>)
where
    OutStream: Stream<Item = Msg> + 'static,
{
    let (current_state, accessor) = state(|| initial_state);
    let accessor_clone = accessor.clone();
    let updater = once(|| Rc::new(update));
    let updater_clone = updater.clone();

    let s = once(|| {
        let (action_producer, action_consumer): (
            mpsc::UnboundedSender<Msg>,
            mpsc::UnboundedReceiver<Msg>,
        ) = mpsc::unbounded();

        let (mut operated_action_producer, operated_action_consumer): (
            mpsc::UnboundedSender<Msg>,
            mpsc::UnboundedReceiver<Msg>,
        ) = mpsc::unbounded();

        let _ = load_once(move || {
            action_consumer.for_each(move |msg| {
                accessor.update(|cur| Some(updater_clone(cur, msg.clone())));
                let _ = operated_action_producer.start_send(msg);
                ready(())
            })
        });

        let _ = load_once(move || {
            operator(operated_action_consumer).for_each(move |msg| {
                accessor_clone.update(|cur| Some(updater(cur, msg.clone())));
                ready(())
            })
        });

        Rc::new(RefCell::new(action_producer))
    });

    (
        current_state,
        Rc::new(move |msg| {
            let _ = s.borrow_mut().start_send(msg);
        }),
    )
}

#[topo::nested]
fn root() -> Div {
    let (ct, dispatch) = state_signal(
        0,
        |state, msg| match msg {
            Msg::Increment => state + 1,
            Msg::Decrement => state - 1,
        },
        |stream| stream.filter(|_| ready(false)),
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
