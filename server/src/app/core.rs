pub enum LineChannelKind {
    User,
    Room,
    Group,
}

pub enum Client {
    Line { id: String, kind: LineChannelKind },
}

pub fn handle_action(action: Action) {
    match action {
        Action::Draw(_client, _meal) => {
            // match client { }
        }
    }
}

pub enum Action {
    Draw(Client, Meal),
}

#[derive(Debug, Clone)]
pub enum Meal {
    Lunch,
    Dinner,
}

#[derive(Debug, Clone)]
pub struct Place {
    pub name: String,
}
