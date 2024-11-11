use crate::State;

#[derive(derive_more::From)]
pub enum UserEvent {
    #[from(winit::event::WindowEvent)]
    Map(winit::event::WindowEvent),
    #[from(State)]
    State(State),
}
