use tiny_tokio_actor::{Actor, ActorRef, Handler, Message, SystemEvent};

use anyhow::Result;

pub async fn actor_edit<T, Query, Set, A, E>(
    ui: &mut egui::Ui,
    actor: ActorRef<E, A>,
    add_contents: impl FnOnce(&mut egui::Ui, &mut T) -> bool)
    -> bool
    where
        Query: Default + Message<Response = T>,
        Set: From<T> + Message<Response = ()>,
        A: Handler<E, Query> + Handler<E, Set>,
        E: SystemEvent, A: Actor<E> {
    let mut value = actor.ask(Query::default()).await.unwrap();
    let response = add_contents(ui, &mut value);
    actor.ask(Set::from(value)).await.unwrap();
    response
}