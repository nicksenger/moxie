use {
    crate::{input::*, Todo},
    moxie_dom::{element, prelude::*, text},
};

#[topo::aware]
fn item_edit_input(todo: Todo, editing: Key<bool>) {
    let todos = topo::Env::expect::<Key<Vec<Todo>>>();

    text_input!(&todo.text.clone(), true, move |value: String| {
        editing.set(false);
        todos.update(|todos| {
            let mut todos = todos.to_vec();
            if let Some(mut todo) = todos.iter_mut().find(|t| t.id == todo.id) {
                todo.text = value;
            }
            Some(todos)
        });
    });
}

#[topo::aware]
fn item_with_buttons(todo: Todo, editing: Key<bool>) {
    let todos = topo::Env::expect::<Key<Vec<Todo>>>();
    let id = todo.id;

    element!("div", |e| e.attr("class", "view").inner(|| {
        element!("input", |e| {
            e.attr("class", "toggle")
                .attr("type", "checkbox")
                .attr("checked", todo.completed)
                .on(
                    move |_: ChangeEvent, todos| {
                        Some(
                            todos
                                .iter()
                                .cloned()
                                .map(move |mut t| {
                                    if t.id == id {
                                        t.completed = !t.completed;
                                        t
                                    } else {
                                        t
                                    }
                                })
                                .collect(),
                        )
                    },
                    todos.clone(),
                );
        });

        element!("label", |e| e
            .on(|_: DoubleClickEvent, _editing| Some(true), editing)
            .inner(|| text!(&todo.text)));

        element!("button", |e| {
            e.attr("class", "destroy").on(
                move |_: ClickEvent, todos| {
                    Some(todos.iter().filter(|t| t.id != id).cloned().collect())
                },
                todos.clone(),
            );
        });
    }));
}

#[topo::aware]
pub fn todo_item(todo: &Todo) {
    let editing = state!(|| false);

    let mut classes = String::new();
    if todo.completed {
        classes.push_str("completed ");
    }
    if *editing {
        classes.push_str("editing");
    }

    element!("li", |e| e.attr("class", classes).inner(|| {
        if *editing {
            item_edit_input!(todo.clone(), editing);
        } else {
            item_with_buttons!(todo.clone(), editing);
        }
    }));
}
